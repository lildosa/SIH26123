use crate::network::fec::AdaptiveBurstTransport;
use crate::network::Network;
use crate::protocol::Envelope;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

#[derive(Debug, Clone)]
pub struct UdpMeshConfig {
    pub multicast_addr: Ipv4Addr,
    pub port: u16,
    pub bind_addr: Ipv4Addr,
}

impl Default for UdpMeshConfig {
    fn default() -> Self {
        Self {
            multicast_addr: Ipv4Addr::new(239, 0, 26, 123),
            port: 26123,
            bind_addr: Ipv4Addr::new(0, 0, 0, 0),
        }
    }
}

pub struct UdpMeshNode {
    socket: Arc<UdpSocket>,
    multicast_dest: SocketAddr,
    rx_buffer: Arc<Mutex<Vec<Envelope>>>,
    recv_task: Option<JoinHandle<()>>,
}

impl UdpMeshNode {
    pub async fn new(config: UdpMeshConfig) -> std::io::Result<Self> {
        let bind_addr = SocketAddrV4::new(config.bind_addr, config.port);
        let socket = UdpSocket::bind(bind_addr).await?;
        socket.join_multicast_v4(config.multicast_addr, config.bind_addr)?;

        let socket = Arc::new(socket);
        let multicast_dest = SocketAddr::V4(SocketAddrV4::new(config.multicast_addr, config.port));
        let rx_buffer = Arc::new(Mutex::new(Vec::new()));

        let socket_clone = socket.clone();
        let rx_buffer_clone = rx_buffer.clone();

        let recv_task = tokio::spawn(async move {
            let mut buf = vec![0u8; 65535];
            loop {
                match socket_clone.recv_from(&mut buf).await {
                    Ok((len, _src)) => {
                        if let Ok(envelope) = serde_json::from_slice::<Envelope>(&buf[..len]) {
                            rx_buffer_clone.lock().await.push(envelope);
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        Ok(Self {
            socket,
            multicast_dest,
            rx_buffer,
            recv_task: Some(recv_task),
        })
    }
}

impl Drop for UdpMeshNode {
    fn drop(&mut self) {
        if let Some(task) = self.recv_task.take() {
            task.abort();
        }
    }
}

#[async_trait::async_trait]
impl Network for UdpMeshNode {
    async fn broadcast(&self, envelope: Envelope) {
        // Adaptive dual-burst: high-priority control envelopes (Intent /
        // Conflict / Yield) are transmitted twice with identical sequence
        // numbers; the receiver dedups the copy for (1 - p^2) survival.
        for frame in AdaptiveBurstTransport::encode(&envelope) {
            if let Ok(bytes) = serde_json::to_vec(&frame) {
                let _ = self.socket.send_to(&bytes, self.multicast_dest).await;
            }
        }
    }

    async fn recv(&self) -> Option<Envelope> {
        let mut buf = self.rx_buffer.lock().await;
        if buf.is_empty() {
            None
        } else {
            Some(buf.remove(0))
        }
    }

    async fn drain(&self) -> Vec<Envelope> {
        let mut buf = self.rx_buffer.lock().await;
        buf.drain(..).collect()
    }
}
