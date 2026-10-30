use core::time::Duration;
use rm_client_core::{LocalUim, SlotState};
use rm_qmi::{LocalQrtrSocket, QrtrAddress, QrtrTransport};
use std::pin::Pin;
use std::future::Future;

/// One card the modem sees.
pub struct ModemSlot {
    pub slot: u8,
    pub present: bool,
    pub ready: bool,
    pub imei: Option<String>,
    pub atr: Option<Vec<u8>>,
}

impl ModemSlot {
    fn new(slot: u8, state: SlotState) -> Self {
        Self {
            slot,
            present: state.present,
            ready: state.ready,
            imei: state.imei,
            atr: state.atr,
        }
    }
}

/// Where a datagram is going, or came from: a node and a port on the bus.
pub struct QrtrSocketAddress {
    pub node: u32,
    pub port: u32,
}

/// One datagram on the bus.
pub struct QrtrSocketDatagram {
    pub node: u32,
    pub port: u32,
    pub data: Vec<u8>,
}

/// A socket abstraction for transport.
pub trait TransportSocket: Send + Sync {
    fn address(&mut self) -> Pin<Box<dyn Future<Output = Result<QrtrSocketAddress, String>> + Send + '_>>;
    fn send(&mut self, to: QrtrSocketAddress, datagram: Vec<u8>) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + '_>>;
    fn receive(&mut self, timeout: Duration) -> Pin<Box<dyn Future<Output = Result<Option<QrtrSocketDatagram>, String>> + Send + '_>>;
}

type QrtrQmi = rm_qmi::QmiClient<QrtrTransport<DartQrtrSocket>>;

pub struct ModemSession {
    client: QrtrQmi,
}

impl ModemSession {
    pub async fn state(&mut self) -> Result<Vec<ModemSlot>, String> {
        let states = self.client.state().await.map_err(describe)?;

        Ok(states
            .into_iter()
            .map(|(slot, state)| ModemSlot::new(slot, state))
            .collect())
    }

    pub async fn reset(&mut self, slot: u8) -> Result<Vec<u8>, String> {
        self.client.reset(slot).await.map_err(describe)
    }

    pub async fn open_channel(&mut self, slot: u8, aid: Vec<u8>) -> Result<u8, String> {
        self.client.open_channel(slot, &aid).await.map_err(describe)
    }

    pub async fn transmit(
        &mut self,
        slot: u8,
        channel: u8,
        apdu: Vec<u8>,
    ) -> Result<Vec<u8>, String> {
        self.client
            .transmit(slot, channel, &apdu)
            .await
            .map_err(describe)
    }

    pub async fn close_channel(&mut self, slot: u8, channel: u8) -> Result<(), String> {
        self.client
            .close_channel(slot, channel)
            .await
            .map_err(describe)
    }
}

struct DartQrtrSocket {
    transport: Box<dyn TransportSocket>,
}

#[derive(Debug)]
pub struct QrtrSocketError(String);

impl core::fmt::Display for QrtrSocketError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.0.fmt(formatter)
    }
}

impl core::error::Error for QrtrSocketError {}

impl LocalQrtrSocket for DartQrtrSocket {
    type Error = QrtrSocketError;

    async fn address(&mut self) -> Result<QrtrAddress, Self::Error> {
        let address = self.transport.address().await.map_err(QrtrSocketError)?;
        Ok(QrtrAddress {
            node: address.node,
            port: address.port,
        })
    }

    async fn send(&mut self, to: QrtrAddress, datagram: &[u8]) -> Result<(), Self::Error> {
        self.transport.send(QrtrSocketAddress { node: to.node, port: to.port }, datagram.to_vec()).await.map_err(QrtrSocketError)
    }

    async fn receive(
        &mut self,
        buffer: &mut [u8],
        timeout: Duration,
    ) -> Result<Option<(usize, QrtrAddress)>, Self::Error> {
        let Some(datagram) = self.transport.receive(timeout).await.map_err(QrtrSocketError)?
        else {
            return Ok(None);
        };

        let copied = datagram.data.len().min(buffer.len());
        buffer[..copied].copy_from_slice(&datagram.data[..copied]);

        Ok(Some((
            copied,
            QrtrAddress {
                node: datagram.node,
                port: datagram.port,
            },
        )))
    }
}

fn describe<E: core::fmt::Display>(error: E) -> String {
    error.to_string()
}
