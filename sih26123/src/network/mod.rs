pub mod faulty;
pub mod in_memory;
pub mod udp_mesh;

pub use faulty::FaultyNetwork;
pub use in_memory::{InMemoryBus, InMemoryNode};
pub use udp_mesh::{UdpMeshConfig, UdpMeshNode};

use crate::protocol::Envelope;

/// Trait abstracting P2P transport.
#[async_trait::async_trait]
pub trait Network: Send + Sync + 'static {
    /// Broadcast an envelope to all peers.
    async fn broadcast(&self, envelope: Envelope);

    /// Receive the next incoming envelope if available.
    async fn recv(&self) -> Option<Envelope>;

    /// Non-blocking drain: returns all currently buffered envelopes.
    async fn drain(&self) -> Vec<Envelope>;
}
