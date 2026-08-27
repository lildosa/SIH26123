use crate::network::Network;
use crate::protocol::{Envelope, RobotId};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};

/// Tick-scoped in-memory message bus for deterministic simulation.
/// Messages broadcast during tick t are staged in `staged_outbox`.
/// Calling `flush_tick()` distributes all staged messages to recipient inboxes for tick t+1.
pub struct InMemoryBus {
    nodes: RwLock<HashMap<RobotId, Arc<Mutex<Vec<Envelope>>>>>,
    staged_outbox: Mutex<Vec<Envelope>>,
}

impl InMemoryBus {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            nodes: RwLock::new(HashMap::new()),
            staged_outbox: Mutex::new(Vec::new()),
        })
    }

    /// Registers a robot node with the bus and returns an `InMemoryNode`.
    pub fn register_node(self: &Arc<Self>, robot_id: RobotId) -> InMemoryNode {
        let inbox = Arc::new(Mutex::new(Vec::new()));
        self.nodes.write().unwrap().insert(robot_id, inbox.clone());
        InMemoryNode {
            robot_id,
            inbox,
            bus: self.clone(),
        }
    }

    /// Flushes all staged messages to recipient inboxes for the next tick.
    /// Messages are not delivered back to their own sender.
    pub fn flush_tick(&self) {
        let mut outbox = self.staged_outbox.lock().unwrap();
        let nodes = self.nodes.read().unwrap();
        for env in outbox.drain(..) {
            for (&recipient_id, inbox) in nodes.iter() {
                if recipient_id != env.sender_id {
                    inbox.lock().unwrap().push(env.clone());
                }
            }
        }
    }
}

pub struct InMemoryNode {
    pub robot_id: RobotId,
    pub inbox: Arc<Mutex<Vec<Envelope>>>,
    pub bus: Arc<InMemoryBus>,
}

#[async_trait::async_trait]
impl Network for InMemoryNode {
    async fn broadcast(&self, envelope: Envelope) {
        self.bus.staged_outbox.lock().unwrap().push(envelope);
    }

    async fn recv(&self) -> Option<Envelope> {
        let mut inbox = self.inbox.lock().unwrap();
        if inbox.is_empty() {
            None
        } else {
            Some(inbox.remove(0))
        }
    }

    async fn drain(&self) -> Vec<Envelope> {
        let mut inbox = self.inbox.lock().unwrap();
        inbox.drain(..).collect()
    }
}
