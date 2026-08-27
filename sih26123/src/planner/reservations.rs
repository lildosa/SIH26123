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

    /// Stores or replaces the IntentRecord for `sender_id` only if `intent.intent_seq > current_seq`.
    /// Returns true if applied, false if rejected as stale or duplicate.
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

    /// Scans every IntentRecord in peer_intents and checks against candidate_path.
    /// Detects both vertex conflicts and edge swaps without flattening.
    pub fn conflicts_with_peers(&self, candidate_path: &[(Pos, Tick)]) -> Vec<PeerConflict> {
        let mut conflicts = Vec::new();
        if candidate_path.is_empty() {
            return conflicts;
        }

        for peer in self.peer_intents.values() {
            // 1. Check Vertex conflicts
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

            // 2. Check Edge-Swap conflicts (head-on swaps)
            // Candidate moves from cand_path[i] to cand_path[i+1] at tick t -> t+1
            // Peer moves from peer_path[j] to peer_path[j+1] at tick t -> t+1
            // Swap occurs if cand[i].pos == peer[j+1].pos && cand[i+1].pos == peer[j].pos
            for c_window in candidate_path.windows(2) {
                let (c_from, c_t_from) = c_window[0];
                let (c_to, c_t_to) = c_window[1];
                if c_t_to != c_t_from + 1 || c_from == c_to {
                    continue; // Not a move or not consecutive tick
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

    /// Constructs explicit SpaceTimeConstraints from all peer intents.
    pub fn build_constraints(&self) -> SpaceTimeConstraints {
        let mut constraints = SpaceTimeConstraints::default();

        for peer in self.peer_intents.values() {
            // Forbid all cells occupied by peer
            for &(p_pos, p_tick) in &peer.path {
                constraints.forbidden_cells.insert((p_pos, p_tick));
            }

            // Forbid reverse edge transitions
            for p_window in peer.path.windows(2) {
                let (p_from, p_t_from) = p_window[0];
                let (p_to, p_t_to) = p_window[1];
                if p_t_to == p_t_from + 1 && p_from != p_to {
                    // Peer moves p_from -> p_to at p_t_from.
                    // Therefore, moving p_to -> p_from at p_t_from is forbidden for us.
                    constraints
                        .forbidden_edges
                        .insert((p_to, p_from, p_t_from));
                }
            }
        }

        constraints
    }
}
