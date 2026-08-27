use crate::protocol::{IntentMsg, RobotId, SeqNum, Tick};
use crate::world::Pos;
use std::collections::{HashMap, HashSet};

/// Record of a peer's latest announced intent.
#[derive(Debug, Clone)]
pub struct IntentRecord {
    pub robot_id: RobotId,
    pub intent_seq: SeqNum,
    pub path: Vec<(Pos, Tick)>,
    pub priority: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictType {
    Vertex,
    EdgeSwap,
}

/// A conflict between candidate path and a specific peer intent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerConflict {
    pub cell: Pos,
    pub tick: Tick,
    pub peer_id: RobotId,
    pub peer_intent_seq: SeqNum,
    pub peer_priority: u64,
    pub conflict_type: ConflictType,
}

/// Explicit set of space-time constraints passed directly into Space-Time A*.
/// Preserves multi-owner claims without flattening.
#[derive(Debug, Clone, Default)]
pub struct SpaceTimeConstraints {
    /// Forbidden (Pos, Tick) cells. Multiple robots may forbid the same cell.
    pub forbidden_cells: HashSet<(Pos, Tick)>,
    /// Forbidden directed edge transitions: (from_pos, to_pos, at_tick).
    /// If another robot moves from B -> A at tick t -> t+1, then moving A -> B at tick t -> t+1 is forbidden.
    pub forbidden_edges: HashSet<(Pos, Pos, Tick)>,
}

#[derive(Debug, Clone)]
pub struct ReservationTable {
    pub own_id: RobotId,
    pub own_path: Vec<(Pos, Tick)>,
    pub peer_intents: HashMap<RobotId, IntentRecord>,
}

impl ReservationTable {
    pub fn new(own_id: RobotId) -> Self {
        Self {
            own_id,
            own_path: Vec::new(),
            peer_intents: HashMap::new(),
        }
    }

    pub fn reserve_own_path(&mut self, path: &[(Pos, Tick)]) {
        self.own_path = path.to_vec();
    }

    pub fn release_own(&mut self) {
        self.own_path.clear();
    }

    pub fn own_path(&self) -> &[(Pos, Tick)] {
        &self.own_path
    }

    pub fn apply_peer_intent(&mut self, sender_id: RobotId, intent: &IntentMsg) -> bool {
        if let Some(existing) = self.peer_intents.get(&sender_id) {
            if intent.intent_seq <= existing.intent_seq {
                return false;
            }
        }
        self.peer_intents.insert(
            sender_id,
            IntentRecord {
                robot_id: sender_id,
                intent_seq: intent.intent_seq,
                path: intent.path.clone(),
                priority: intent.priority,
            },
        );
        true
    }

    pub fn remove_peer(&mut self, peer_id: RobotId) {
        self.peer_intents.remove(&peer_id);
    }

    pub fn get_peer_intent(&self, peer_id: RobotId) -> Option<&IntentRecord> {
        self.peer_intents.get(&peer_id)
    }

    pub fn all_peer_intents(&self) -> impl Iterator<Item = &IntentRecord> {
        self.peer_intents.values()
    }

    pub fn conflicts_with_peers(&self, candidate_path: &[(Pos, Tick)]) -> Vec<PeerConflict> {
        let mut conflicts = Vec::new();
        if candidate_path.is_empty() {
            return conflicts;
        }

        for peer in self.peer_intents.values() {
            // 1. Vertex conflicts
            for (c_pos, c_tick) in candidate_path {
                for (p_pos, p_tick) in &peer.path {
                    if c_pos == p_pos && c_tick == p_tick {
                        conflicts.push(PeerConflict {
                            cell: *c_pos,
                            tick: *c_tick,
                            peer_id: peer.robot_id,
                            peer_intent_seq: peer.intent_seq,
                            peer_priority: peer.priority,
                            conflict_type: ConflictType::Vertex,
                        });
                    }
                }
            }

            // 2. Edge-Swap conflicts
            for c_window in candidate_path.windows(2) {
                let (c_from, c_t_from) = c_window[0];
                let (c_to, c_t_to) = c_window[1];
                if c_t_to != c_t_from + 1 || c_from == c_to {
                    continue;
                }

                for p_window in peer.path.windows(2) {
                    let (p_from, p_t_from) = p_window[0];
                    let (p_to, p_t_to) = p_window[1];
                    if p_t_to != p_t_from + 1 || p_from == p_to {
                        continue;
                    }

                    if c_t_from == p_t_from && c_from == p_to && c_to == p_from {
                        conflicts.push(PeerConflict {
                            cell: c_to,
                            tick: c_t_to,
                            peer_id: peer.robot_id,
                            peer_intent_seq: peer.intent_seq,
                            peer_priority: peer.priority,
                            conflict_type: ConflictType::EdgeSwap,
                        });
                    }
                }
            }
        }

        conflicts
    }

    pub fn build_constraints(&self) -> SpaceTimeConstraints {
        let mut constraints = SpaceTimeConstraints::default();

        for peer in self.peer_intents.values() {
            for &(p_pos, p_tick) in &peer.path {
                constraints.forbidden_cells.insert((p_pos, p_tick));
            }

            for p_window in peer.path.windows(2) {
                let (p_from, p_t_from) = p_window[0];
                let (p_to, p_t_to) = p_window[1];
                if p_t_to == p_t_from + 1 && p_from != p_to {
                    constraints
                        .forbidden_edges
                        .insert((p_to, p_from, p_t_from));
                }
            }
        }

        constraints
    }

    pub fn build_constraints_with_stationary(
        &self,
        peer_poses: &HashMap<RobotId, (Pos, Tick)>,
        current_tick: Tick,
        horizon: Tick,
    ) -> SpaceTimeConstraints {
        let mut constraints = self.build_constraints();

        for (&peer_id, &(peer_pos, _)) in peer_poses {
            if peer_id == self.own_id {
                continue;
            }

            // If peer has no announced future steps, it is currently sitting at peer_pos
            let future_path_steps: Vec<(Pos, Tick)> = self
                .peer_intents
                .get(&peer_id)
                .map(|rec| {
                    rec.path
                        .iter()
                        .filter(|(_, t)| *t >= current_tick)
                        .copied()
                        .collect()
                })
                .unwrap_or_default();

            if future_path_steps.is_empty() {
                for t in current_tick..=(current_tick + horizon) {
                    constraints.forbidden_cells.insert((peer_pos, t));
                }
            } else {
                // If peer has a future path, where does it end?
                // After reaching its path endpoint, it sits at that endpoint for the remaining horizon!
                if let Some(&(end_pos, end_tick)) = future_path_steps.last() {
                    for t in (end_tick + 1)..=(current_tick + horizon) {
                        constraints.forbidden_cells.insert((end_pos, t));
                    }
                }
            }
        }

        constraints
    }
}
