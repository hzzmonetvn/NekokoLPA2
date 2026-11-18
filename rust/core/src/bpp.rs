//! GSMA SGP.22 Bound Profile Package (BPP) segmentation, session cancellation,
//! eUICC memory reset payload constants, and NXP proprietary memory TLV parsing.

use serde::{Deserialize, Serialize};

use crate::asn1::{encode_tlv, Asn1Error, Asn1Reader};

/// Standard `lpac`-equivalent `ES10c.EuiccMemoryReset` APDU data (`BF34 04 82 02 05 E0`),
/// deleting operational and field-loaded test profiles and resetting the default SM-DP+ address.
pub const EUICC_MEMORY_RESET_APDU_DATA: &[u8] = &[0xBF, 0x34, 0x04, 0x82, 0x02, 0x05, 0xE0];

/// Memory metrics parsed from NXP proprietary `GET DATA (80 CA 00 FE)` tag `DF25`.
#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct NxpMemoryInfo {
    pub free_non_volatile_memory: Option<u64>,
    pub free_volatile_memory: Option<u64>,
}

/// Decomposes a DER-encoded GSMA SGP.22 `BoundProfilePackage` (`0xBF36`) into ordered
/// `STORE DATA` segments ready for APDU transport.
///
/// Follows the SGP.22 ES8+/ES10b segmentation rules:
/// 1. Outer `0xBF36` tag+length combined with `initialiseSecureChannelRequest` (`0xBF23`).
/// 2. `firstSequenceOf87` (`0xA0`) header+first `0x87` TLV, followed by remaining `0x87` TLVs.
/// 3. `sequenceOf88` (`0xA1`) tag+length header followed by each `0x88` TLV individually.
/// 4. `secondSequenceOf87` (`0xA2`) header+first `0x87` TLV, followed by remaining `0x87` TLVs.
/// 5. `sequenceOf86` (`0xA3`) tag+length header followed by each `0x86` TLV individually.
///
/// # Errors
/// Returns [`Asn1Error`] if `bpp_der` is truncated, malformed, or does not begin with tag `0xBF36`.
pub fn decompose_bpp(bpp_der: &[u8]) -> Result<Vec<Vec<u8>>, Asn1Error> {
    let mut reader = Asn1Reader::new(bpp_der);
    let root = reader.read_next()?.ok_or(Asn1Error::UnexpectedEof {
        offset: 0,
        needed: 1,
    })?;

    if root.tag != 0xBF36 {
        return Err(Asn1Error::UnexpectedTag {
            expected: 0xBF36,
            actual: root.tag,
        });
    }

    let root_tl_len = root.raw_bytes.len().saturating_sub(root.value.len());
    let root_tl = &root.raw_bytes[..root_tl_len];
    let children = root.children()?;

    let mut segments = Vec::new();
    let start_idx = if children.first().is_some_and(|c| c.tag == 0xBF23) {
        let mut first = Vec::with_capacity(root_tl.len() + children[0].raw_bytes.len());
        first.extend_from_slice(root_tl);
        first.extend_from_slice(&children[0].raw_bytes);
        segments.push(first);
        1
    } else {
        segments.push(root_tl.to_vec());
        0
    };

    for seq in &children[start_idx..] {
        let seq_tl_len = seq.raw_bytes.len().saturating_sub(seq.value.len());
        let seq_tl = &seq.raw_bytes[..seq_tl_len];
        let elements = seq.children()?;

        if elements.is_empty() {
            segments.push(seq.raw_bytes.clone());
            continue;
        }

        match seq.tag {
            0xA0 | 0xA2 => {
                let mut combined = Vec::with_capacity(seq_tl.len() + elements[0].raw_bytes.len());
                combined.extend_from_slice(seq_tl);
                combined.extend_from_slice(&elements[0].raw_bytes);
                segments.push(combined);
                for element in &elements[1..] {
                    segments.push(element.raw_bytes.clone());
                }
            }
            0xA1 | 0xA3 => {
                segments.push(seq_tl.to_vec());
                for element in elements {
                    segments.push(element.raw_bytes);
                }
            }
            _ => {
                segments.push(seq.raw_bytes.clone());
            }
        }
    }

    Ok(segments)
}

/// Encodes a GSMA SGP.22 ES10b `CancelSessionRequest` (`0xBF41`) in ASN.1 DER format.
#[must_use]
pub fn build_cancel_session_request_der(
    transaction_id: &[u8],
    euicc_cancel_response_or_reason: u8,
) -> Vec<u8> {
    let tx_tlv = encode_tlv(0x80, transaction_id);
    let reason_tlv = encode_tlv(0x81, &[euicc_cancel_response_or_reason]);
    let mut inner = Vec::with_capacity(tx_tlv.len() + reason_tlv.len());
    inner.extend_from_slice(&tx_tlv);
    inner.extend_from_slice(&reason_tlv);
    encode_tlv(0xBF41, &inner)
}

/// Parses NXP proprietary `GET DATA (80 CA 00 FE)` `DF25` memory response bytes.
///
/// Accepts either an outer `0xFE` wrapper containing `0xDF25`, a bare `0xDF25` TLV,
/// or a raw 5-byte `FE <len> DF 25 <len>` prefix followed by 6-byte big-endian records.
#[must_use]
pub fn parse_nxp_memory_tlv(bytes: &[u8]) -> Option<NxpMemoryInfo> {
    parse_nxp_memory_tlv_internal(bytes, None, false)
}

/// Parses NXP proprietary `DF25` memory response bytes and deducts firmware-specific
/// reserved overhead (`b14` for `98.0.22` / `98.16.25`, or `b3 * 16` otherwise).
#[must_use]
pub fn parse_nxp_memory_tlv_with_firmware(
    bytes: &[u8],
    firmware_version: &str,
) -> Option<NxpMemoryInfo> {
    parse_nxp_memory_tlv_internal(bytes, Some(firmware_version), true)
}

fn parse_nxp_memory_tlv_internal(
    bytes: &[u8],
    firmware_version: Option<&str>,
    subtract_overhead: bool,
) -> Option<NxpMemoryInfo> {
    let payload = extract_nxp_df25_payload(bytes)?;
    if payload.len() < 6 {
        return None;
    }

    let mut available_nvm: Option<u64> = None;
    let mut volatile_ram: Option<u64> = None;
    let mut b3_value: u64 = 0;
    let mut b14_value: u64 = 0;

    for chunk in payload.chunks_exact(6) {
        let kind = chunk[0];
        let val = u64::from(u32::from_be_bytes([chunk[2], chunk[3], chunk[4], chunk[5]]));
        match kind {
            0 => {
                available_nvm = Some(available_nvm.unwrap_or(0).saturating_add(val));
            }
            1 => {
                volatile_ram = Some(val);
            }
            3 => {
                b3_value = val;
            }
            14 => {
                b14_value = val;
            }
            _ => {}
        }
    }

    if available_nvm.is_none() && volatile_ram.is_none() {
        return None;
    }

    if subtract_overhead {
        if let Some(nvm) = available_nvm.as_mut() {
            let overhead = match firmware_version {
                Some("98.0.22" | "98.16.25") => b14_value,
                _ => b3_value.saturating_mul(16),
            };
            *nvm = nvm.saturating_sub(overhead);
        }
    }

    Some(NxpMemoryInfo {
        free_non_volatile_memory: available_nvm,
        free_volatile_memory: volatile_ram,
    })
}

fn extract_nxp_df25_payload(bytes: &[u8]) -> Option<Vec<u8>> {
    if let Ok(Some(node)) = Asn1Reader::new(bytes).read_next() {
        if node.tag == 0xFE {
            if let Some(df25) = node.find_child(0xDF25) {
                return Some(df25.value);
            }
            return None;
        }
        if node.tag == 0xDF25 {
            return Some(node.value);
        }
    }

    if bytes.len() >= 11 && bytes[0] == 0xFE && bytes[2] == 0xDF && bytes[3] == 0x25 {
        return Some(bytes[5..].to_vec());
    }

    None
}
