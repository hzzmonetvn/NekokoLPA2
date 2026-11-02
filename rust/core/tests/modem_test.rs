use core::time::Duration;
use nlpa_core::{
    DartQrtrSocket, ModemSession, ModemSlot, QrtrSocketAddress, QrtrSocketDatagram,
    QrtrSocketError, TransportSocket,
};
use rm_client_core::SlotState;
use rm_qmi::{LocalQrtrSocket, QrtrAddress};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

type SentEntries = Arc<Mutex<Vec<(QrtrSocketAddress, Vec<u8>)>>>;

/// In-memory mock transport socket for testing without physical Qualcomm hardware.
#[derive(Clone, Default)]
struct MockTransportSocket {
    address: QrtrSocketAddress,
    sent: SentEntries,
    to_receive: Arc<Mutex<Option<QrtrSocketDatagram>>>,
    fail_next: Arc<Mutex<bool>>,
}

impl MockTransportSocket {
    fn new(node: u32, port: u32) -> Self {
        Self {
            address: QrtrSocketAddress { node, port },
            sent: Arc::new(Mutex::new(Vec::new())),
            to_receive: Arc::new(Mutex::new(None)),
            fail_next: Arc::new(Mutex::new(false)),
        }
    }

    fn queue_receive(&self, datagram: QrtrSocketDatagram) {
        *self.to_receive.lock().unwrap() = Some(datagram);
    }

    fn set_fail_next(&self, fail: bool) {
        *self.fail_next.lock().unwrap() = fail;
    }

    fn get_sent(&self) -> Vec<(QrtrSocketAddress, Vec<u8>)> {
        self.sent.lock().unwrap().clone()
    }
}

impl TransportSocket for MockTransportSocket {
    fn address(
        &mut self,
    ) -> Pin<Box<dyn Future<Output = Result<QrtrSocketAddress, String>> + Send + '_>> {
        let fail = *self.fail_next.lock().unwrap();
        let addr = self.address;
        Box::pin(async move {
            if fail {
                Err("mock address failure".to_string())
            } else {
                Ok(addr)
            }
        })
    }

    fn send(
        &mut self,
        to: QrtrSocketAddress,
        datagram: Vec<u8>,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + '_>> {
        let fail = *self.fail_next.lock().unwrap();
        let sent = Arc::clone(&self.sent);
        Box::pin(async move {
            if fail {
                Err("mock send failure".to_string())
            } else {
                sent.lock().unwrap().push((to, datagram));
                Ok(())
            }
        })
    }

    fn receive(
        &mut self,
        _timeout: Duration,
    ) -> Pin<Box<dyn Future<Output = Result<Option<QrtrSocketDatagram>, String>> + Send + '_>> {
        let fail = *self.fail_next.lock().unwrap();
        let to_receive = Arc::clone(&self.to_receive);
        Box::pin(async move {
            if fail {
                Err("mock receive failure".to_string())
            } else {
                let item = to_receive.lock().unwrap().take();
                Ok(item)
            }
        })
    }
}

#[test]
fn test_modem_slot_mapping() {
    let state = SlotState {
        present: true,
        ready: true,
        imei: Some("860123456789012".to_string()),
        atr: Some(vec![0x3B, 0x9F, 0x96, 0x80]),
        extensions: Default::default(),
    };

    let slot = ModemSlot::new(1, state);

    assert_eq!(slot.slot, 1);
    assert!(slot.present);
    assert!(slot.ready);
    assert_eq!(slot.imei.as_deref(), Some("860123456789012"));
    assert_eq!(slot.atr.as_deref(), Some(&[0x3B, 0x9F, 0x96, 0x80][..]));
}

#[test]
fn test_modem_slot_empty_state() {
    let state = SlotState {
        present: false,
        ready: false,
        imei: None,
        atr: None,
        extensions: Default::default(),
    };

    let slot = ModemSlot::new(2, state);

    assert_eq!(slot.slot, 2);
    assert!(!slot.present);
    assert!(!slot.ready);
    assert_eq!(slot.imei, None);
    assert_eq!(slot.atr, None);
}

#[test]
fn test_qrtr_socket_error_display() {
    let err = QrtrSocketError("bus timeout reached".to_string());
    assert_eq!(format!("{}", err), "bus timeout reached");
    assert_eq!(
        format!("{:?}", err),
        "QrtrSocketError(\"bus timeout reached\")"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn test_dart_qrtr_socket_address() {
    let mock = MockTransportSocket::new(42, 100);
    let mut socket = DartQrtrSocket {
        transport: Box::new(mock.clone()),
    };

    let addr = socket.address().await.expect("should resolve address");
    assert_eq!(addr.node, 42);
    assert_eq!(addr.port, 100);

    mock.set_fail_next(true);
    let err = socket.address().await.expect_err("should return error");
    assert_eq!(err.0, "mock address failure");
}

#[tokio::test(flavor = "current_thread")]
async fn test_dart_qrtr_socket_send() {
    let mock = MockTransportSocket::new(1, 2);
    let mut socket = DartQrtrSocket {
        transport: Box::new(mock.clone()),
    };

    let dest = QrtrAddress { node: 5, port: 8 };
    let payload = vec![0x01, 0x02, 0x03, 0x04];
    socket
        .send(dest, &payload)
        .await
        .expect("send should succeed");

    let sent = mock.get_sent();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].0.node, 5);
    assert_eq!(sent[0].0.port, 8);
    assert_eq!(sent[0].1, payload);

    mock.set_fail_next(true);
    let err = socket
        .send(dest, &payload)
        .await
        .expect_err("send should fail");
    assert_eq!(err.0, "mock send failure");
}

#[tokio::test(flavor = "current_thread")]
async fn test_dart_qrtr_socket_receive() {
    let mock = MockTransportSocket::new(1, 2);
    let mut socket = DartQrtrSocket {
        transport: Box::new(mock.clone()),
    };

    let mut buf = [0u8; 16];
    let res = socket
        .receive(&mut buf, Duration::from_millis(50))
        .await
        .expect("empty receive should be ok");
    assert_eq!(res, None);

    mock.queue_receive(QrtrSocketDatagram {
        node: 10,
        port: 20,
        data: vec![0xAA, 0xBB, 0xCC],
    });

    let res = socket
        .receive(&mut buf, Duration::from_millis(50))
        .await
        .expect("receive should succeed");
    assert!(res.is_some());
    let (n, addr) = res.unwrap();
    assert_eq!(n, 3);
    assert_eq!(addr.node, 10);
    assert_eq!(addr.port, 20);
    assert_eq!(&buf[..3], &[0xAA, 0xBB, 0xCC]);

    mock.set_fail_next(true);
    let err = socket
        .receive(&mut buf, Duration::from_millis(50))
        .await
        .expect_err("receive should fail");
    assert_eq!(err.0, "mock receive failure");
}

#[test]
fn test_modem_session_construct() {
    let mock = MockTransportSocket::new(1, 1);
    let _session = ModemSession::new(Box::new(mock));
}
