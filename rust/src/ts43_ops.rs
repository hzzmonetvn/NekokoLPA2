//! GSMA TS.43 carrier eSIM transfer bridge over `HostTransport` HTTP and USIM APDU channels.

use std::collections::{BTreeMap, HashMap};
use std::error::Error;
use std::sync::{Arc, Mutex};

use nlpa_core::channel::OmapiSafetyLatch;
use nlpa_core::crypto::{base64_decode, base64_encode};
use nlpa_core::hex::bytes_to_hex;
use nlpa_core::lpa::ts43::{
    decode_swapped_bcd, fid, group_identifier_string, AkaRequest, AkaResponse, Device, Endpoint,
    Entitlement, EntitlementChannel, EntitlementRecord, Identity, Imsi, SimType, SourceSimProvider,
    Ts43Client, Ts43Config, Ts43HttpError, Ts43HttpRequest, Ts43HttpResponse, Ts43Method,
    Ts43Transport, UsimFiles,
};
use nlpa_core::storage::PersistedState;

use crate::host::{HostClient, HostTransport, HttpRequestArgs};
use crate::readers::{CardChannel, DiscoveredReader};

/// Standard fallback USIM AID prefix hex when `EF_DIR` discovery is unavailable.
const DEFAULT_USIM_AID_HEX: &str = "A0000000871002";

/// `Ts43Transport` backed by the Android host `http.request` RPC.
pub struct HostTs43Transport {
    pub host: Arc<dyn HostTransport>,
    pub proxy_url: Option<String>,
}

impl Ts43Transport for HostTs43Transport {
    fn do_raw(
        &self,
        request: &Ts43HttpRequest,
    ) -> Result<Ts43HttpResponse, Box<dyn Error + Send + Sync>> {
        let client = HostClient::new(self.host.as_ref());
        let mut headers = BTreeMap::new();
        for (k, v) in &request.headers {
            headers.insert(k.clone(), v.clone());
        }
        if !request.cookies.is_empty() {
            headers.insert("Cookie".into(), request.cookies.join("; "));
        }
        let method = match request.method {
            Ts43Method::Get => "GET",
            Ts43Method::Post => "POST",
        };
        let body_base64 = if request.body.is_empty() {
            None
        } else {
            Some(base64_encode(&request.body))
        };

        let resp = client.http_request(&HttpRequestArgs {
            method: method.into(),
            url: request.url.clone(),
            headers,
            body_base64,
            proxy_url: self.proxy_url.clone(),
            timeout_ms: 30_000,
            insecure_tls: false,
        })?;

        let body = base64_decode(&resp.body_base64)?;
        if !(200..300).contains(&resp.status) {
            let err_headers: HashMap<String, String> = resp.headers.into_iter().collect();
            return Err(Box::new(Ts43HttpError::new(resp.status, err_headers, body)));
        }

        Ok(Ts43HttpResponse::new(body, resp.set_cookies))
    }
}

/// USIM-backed [`SourceSimProvider`] executing EF reads and 3G EAP-AKA over [`CardChannel`].
pub struct CardSourceSim {
    pub host: Arc<dyn HostTransport>,
    pub reader: DiscoveredReader,
    pub state: PersistedState,
    pub omapi_latch: Arc<Mutex<OmapiSafetyLatch>>,
    pub fallback_iccid: Option<String>,
}

impl CardSourceSim {
    fn open_usim(&self) -> Result<CardChannel, Box<dyn Error + Send + Sync>> {
        let aids = vec![
            "A0000000871002FF49FFFF89040B00FF".to_string(),
            DEFAULT_USIM_AID_HEX.to_string(),
        ];
        let borrowed = match self.reader.slot_index {
            Some(2) => self.state.se_hal_borrowed_slot2,
            _ => self.state.se_hal_borrowed_slot1,
        };
        let ch = CardChannel::open(
            Arc::clone(&self.host),
            &self.reader,
            &aids,
            usize::from(self.state.mss.max(32)),
            &self.omapi_latch,
            borrowed,
            self.state.proxy_url.clone(),
        )?;
        Ok(ch)
    }
}

impl SourceSimProvider for CardSourceSim {
    fn identity(&self) -> Result<Identity, Box<dyn Error + Send + Sync>> {
        let mut ch = self.open_usim()?;
        let iccid_bytes = read_ef_binary(&mut ch, UsimFiles::EF_ICCID, 10).unwrap_or_default();
        let iccid = if iccid_bytes.is_empty() {
            self.fallback_iccid.clone().unwrap_or_default()
        } else {
            decode_swapped_bcd(&iccid_bytes, true)
        };
        let imsi_bytes = read_ef_binary(&mut ch, UsimFiles::EF_IMSI, 9)?;
        let imsi = Imsi::from_bytes(&imsi_bytes)?;
        let ad_bytes =
            read_ef_binary(&mut ch, UsimFiles::EF_AD, 4).unwrap_or_else(|_| vec![0, 0, 0, 2]);
        let gid1_bytes = read_ef_binary(&mut ch, UsimFiles::EF_GID1, 16).unwrap_or_default();
        let gid1 = if gid1_bytes.is_empty() {
            String::new()
        } else {
            group_identifier_string(&gid1_bytes)
        };
        let _ = ch.close();

        let imsi_str = imsi.as_str().to_string();
        Ok(Identity::derive(iccid, imsi_str, gid1, &ad_bytes)?)
    }

    fn authenticate_aka(
        &self,
        request: &AkaRequest,
    ) -> Result<AkaResponse, Box<dyn Error + Send + Sync>> {
        let mut ch = self.open_usim()?;
        let payload = request.marshal();
        let mut apdu = Vec::with_capacity(5 + payload.len() + 1);
        apdu.push(0x00);
        apdu.push(0x88);
        apdu.push(0x00);
        apdu.push(0x81);
        apdu.push(u8::try_from(payload.len()).unwrap_or(0));
        apdu.extend_from_slice(&payload);
        apdu.push(0x00);

        let resp = ch.transmit_raw(&apdu)?;
        let _ = ch.close();
        if resp.len() < 2 {
            return Err("Short USIM AUTHENTICATE response".into());
        }
        let body = &resp[..resp.len() - 2];
        Ok(AkaResponse::from_bytes(body)?)
    }
}

fn read_ef_binary(
    ch: &mut CardChannel,
    ef: u16,
    len: u8,
) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
    let f = fid(ef);
    let sel_apdu = [0x00, 0xA4, 0x00, 0x04, 0x02, f[0], f[1], 0x00];
    let sel_resp = ch.transmit_raw(&sel_apdu)?;
    ensure_sw_ok(&sel_resp, "SELECT EF")?;
    let read_apdu = [0x00, 0xB0, 0x00, 0x00, len];
    let read_resp = ch.transmit_raw(&read_apdu)?;
    ensure_sw_ok(&read_resp, "READ BINARY")?;
    Ok(read_resp[..read_resp.len() - 2].to_vec())
}

fn ensure_sw_ok(resp: &[u8], op: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
    if resp.len() < 2 {
        return Err(format!("{op}: truncated response").into());
    }
    let sw1 = resp[resp.len() - 2];
    let sw2 = resp[resp.len() - 1];
    if (sw1 == 0x90 && sw2 == 0x00) || sw1 == 0x91 {
        Ok(())
    } else {
        Err(format!("{op} returned SW={sw1:02X}{sw2:02X} ({})", bytes_to_hex(resp)).into())
    }
}

/// Parses bundled carrier TS.43 entitlement JSON records.
#[must_use]
pub fn parse_entitlement_database(json: &str) -> Vec<EntitlementRecord> {
    if json.trim().is_empty() {
        return Vec::new();
    }
    nlpa_core::lpa::ts43::parse_entitlement_records(json).unwrap_or_default()
}

/// Builds a configured [`Ts43Client`] for initiating or continuing a carrier transfer.
#[allow(clippy::too_many_arguments)]
#[must_use]
pub fn build_ts43_client(
    host: Arc<dyn HostTransport>,
    reader: DiscoveredReader,
    state: PersistedState,
    omapi_latch: Arc<Mutex<OmapiSafetyLatch>>,
    entitlement_records: Vec<EntitlementRecord>,
    manual_server: &str,
    source_iccid: Option<String>,
    target_eid: &str,
    app_version: &str,
) -> Ts43Client {
    let entitlement = Entitlement {
        server: if manual_server.trim().is_empty() {
            String::new()
        } else {
            manual_server.trim().to_string()
        },
        channel: EntitlementChannel::Google,
        source_sim_type: SimType::Esim,
        ..Entitlement::default()
    };

    let source_dev = Device {
        imei: state.device_imei.clone(),
        iccid: source_iccid.clone().unwrap_or_default(),
        ..Device::default()
    };
    let target_dev = Device {
        imei: state.device_imei.clone(),
        eid: target_eid.to_string(),
        ..Device::default()
    };

    let source_sim = Arc::new(CardSourceSim {
        host: Arc::clone(&host),
        reader,
        state: state.clone(),
        omapi_latch,
        fallback_iccid: source_iccid,
    });

    Ts43Client::new(Ts43Config {
        transport: Arc::new(HostTs43Transport {
            host,
            proxy_url: state.proxy_url,
        }),
        source_sim: Some(source_sim),
        entitlement_records: Some(entitlement_records),
        entitlement,
        source: Endpoint { device: source_dev },
        target: Endpoint { device: target_dev },
        companion: Device::default(),
        app_version: app_version.to_string(),
        client_ts43: String::new(),
        realm: String::new(),
    })
}
