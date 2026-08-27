use crate::auction::{compute_bid_cost, Auction, AuctionConfig};
use crate::negotiator::{resolve_deadlocks, should_yield, WaitForGraph};
use crate::network::Network;
use crate::node::environment::Environment;
use crate::node::state::RobotState;
use crate::planner::{plan, ReservationTable};
use crate::protocol::{
    AuctionOpenMsg, AwardMsg, BidMsg, ConflictMsg, Envelope, FleetMessage, HeartbeatMsg, IntentMsg,
    PoseMsg, RobotId, RobotStatus, SeqNum, TaskId, TaskState, TaskStatusMsg, Tick, YieldMsg,
};
use crate::world::{Cell, GridMap, Pos};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::watch;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RobotTelemetry {
    pub id: RobotId,
    pub pos: Pos,
    pub tick: Tick,
    pub battery: f32,
    pub status: RobotStatus,
    pub path: Vec<Pos>,
    pub task: Option<TaskId>,
}

pub struct RobotActor {
    pub id: RobotId,
    pub pos: Pos,
    pub battery: f32,
    pub state: RobotState,

    pub next_seq: SeqNum,
    pub current_intent_seq: SeqNum,
    pub current_intent_priority: u64,

    pub last_seq_seen: HashMap<RobotId, SeqNum>,

    pub local_reservations: ReservationTable,
    pub local_wfg: WaitForGraph,
    pub local_obstacles: HashSet<Pos>,

    pub peer_poses: HashMap<RobotId, (Pos, Tick)>,
    pub last_heartbeats: HashMap<RobotId, Tick>,

    pub assigned_task: Option<(TaskId, Pos, Pos)>,
    pub known_tasks: HashMap<TaskId, TaskStatusMsg>,
    pub pending_auctions: HashMap<TaskId, Auction>,
    pub auction_config: AuctionConfig,

    pub grid: Arc<GridMap>,
    pub network: Arc<dyn Network>,
    pub environment: Arc<dyn Environment>,

    pub current_tick: Tick,
    pub alive: bool,

    pub outbox: Vec<Envelope>,
    pub desired_next_pos: Option<Pos>,

    pub telemetry_tx: watch::Sender<RobotTelemetry>,
    pub telemetry_rx: watch::Receiver<RobotTelemetry>,
}

impl RobotActor {
    pub fn new(
        id: RobotId,
        pos: Pos,
        grid: Arc<GridMap>,
        network: Arc<dyn Network>,
        environment: Arc<dyn Environment>,
    ) -> Self {
        let initial_telemetry = RobotTelemetry {
            id,
            pos,
            tick: 0,
            battery: 1.0,
            status: RobotStatus::Idle,
            path: Vec::new(),
            task: None,
        };
        let (telemetry_tx, telemetry_rx) = watch::channel(initial_telemetry);

        Self {
            id,
            pos,
            battery: 1.0,
            state: RobotState::Idle,

            next_seq: 1,
            current_intent_seq: 0,
            current_intent_priority: id as u64,

            last_seq_seen: HashMap::new(),

            local_reservations: ReservationTable::new(id),
            local_wfg: WaitForGraph::new(),
            local_obstacles: HashSet::new(),

            peer_poses: HashMap::new(),
            last_heartbeats: HashMap::new(),

            assigned_task: None,
            known_tasks: HashMap::new(),
            pending_auctions: HashMap::new(),
            auction_config: AuctionConfig::default(),

            grid,
            network,
            environment,

            current_tick: 0,
            alive: true,

            outbox: Vec::new(),
            desired_next_pos: None,

            telemetry_tx,
            telemetry_rx,
        }
    }

    /// Enqueues an outgoing envelope with an auto-incremented monotonic sequence number.
    pub fn send(&mut self, payload: FleetMessage) {
        let env = Envelope {
            sender_id: self.id,
            seq: self.next_seq,
            payload,
        };
        self.next_seq += 1;
        self.outbox.push(env);
    }

    /// Phase 1: Local sensing of dynamic obstacles within range.
    pub fn sense_phase(&mut self) {
        if !self.alive {
            return;
        }

        let sensed = self.environment.sense_obstacles(self.pos, 3);
        let mut path_blocked = false;

        for obs in sensed {
            if self.local_obstacles.insert(obs) {
                if let RobotState::Moving { ref path, step_index } = self.state {
                    if path[step_index..].iter().any(|(p, _)| *p == obs) {
                        path_blocked = true;
                    }
                }
            }
        }

        if path_blocked {
            if let Some((task_id, pickup, dropoff)) = self.assigned_task {
                self.state = RobotState::Replanning {
                    task_id,
                    pickup,
                    dropoff,
                };
            }
        }
    }

    /// Phase 2: Inbox processing with dedup, state machine decision, and outbox population.
    pub fn decide_phase(&mut self, inbox: Vec<Envelope>) {
        if !self.alive {
            return;
        }
        self.current_tick += 1;

        // 2a. Process messages with deduplication
        for env in inbox {
            if env.sender_id == self.id {
                continue;
            }
            if let Some(&last_seq) = self.last_seq_seen.get(&env.sender_id) {
                if env.seq <= last_seq {
                    continue;
                }
            }
            self.last_seq_seen.insert(env.sender_id, env.seq);

            match env.payload {
                FleetMessage::Pose(m) => {
                    self.peer_poses.insert(env.sender_id, (m.pos, m.tick));
                }
                FleetMessage::Heartbeat(m) => {
                    self.last_heartbeats.insert(env.sender_id, m.tick);
                }
                FleetMessage::Intent(m) => {
                    let applied = self.local_reservations.apply_peer_intent(env.sender_id, &m);
                    if applied {
                        if let RobotState::Moving { ref path, step_index } = self.state {
                            let remaining: Vec<(Pos, Tick)> = path[step_index..]
                                .iter()
                                .enumerate()
                                .map(|(offset, (pos, _))| (*pos, self.current_tick + offset as u64))
                                .collect();

                            let conflicts = self.local_reservations.conflicts_with_peers(&remaining);
                            for c in conflicts {
                                if c.peer_id == env.sender_id {
                                    self.handle_detected_conflict(&c);
                                }
                            }
                        }
                    }
                }
                FleetMessage::Conflict(m) => {
                    if m.challenged_id == self.id && m.challenged_intent_seq == self.current_intent_seq {
                        if should_yield(
                            self.id,
                            self.current_intent_priority,
                            env.sender_id,
                            m.challenger_priority,
                        ) {
                            self.send(FleetMessage::Yield(YieldMsg {
                                yielded_intent_seq: self.current_intent_seq,
                                to_robot: env.sender_id,
                                conflicting_cell: m.conflicting_cell,
                                conflicting_tick: m.conflicting_tick,
                            }));
                            if let Some((task_id, pickup, dropoff)) = self.assigned_task {
                                self.state = RobotState::Replanning {
                                    task_id,
                                    pickup,
                                    dropoff,
                                };
                            }
                        }
                    }
                }
                FleetMessage::Yield(m) => {
                    if m.to_robot == self.id {
                        self.local_wfg.remove_robot(env.sender_id);
                    }
                }
                FleetMessage::TaskStatus(m) => {
                    self.known_tasks.insert(m.task_id, m);
                }
                FleetMessage::AuctionOpen(m) => {
                    self.known_tasks.entry(m.task_id).or_insert(TaskStatusMsg {
                        task_id: m.task_id,
                        pickup: m.pickup,
                        dropoff: m.dropoff,
                        assigned_to: None,
                        status: TaskState::Open,
                    });

                    self.pending_auctions.entry(m.task_id).or_insert_with(|| {
                        Auction::new(m.task_id, m.pickup, m.dropoff, self.current_tick, m.deadline_tick.saturating_sub(self.current_tick).max(1))
                    });

                    if self.state == RobotState::Idle && self.assigned_task.is_none() {
                        let cost = compute_bid_cost(
                            &self.auction_config,
                            self.pos,
                            self.battery,
                            m.pickup,
                            m.dropoff,
                            0.0,
                            0,
                            Some(m.deadline_tick),
                            self.current_tick,
                        );
                        self.send(FleetMessage::Bid(BidMsg {
                            task_id: m.task_id,
                            cost,
                        }));
                    }
                }
                FleetMessage::Bid(m) => {
                    if let Some(auction) = self.pending_auctions.get_mut(&m.task_id) {
                        auction.add_bid(env.sender_id, &m);
                    }
                }
                FleetMessage::Award(m) => {
                    self.pending_auctions.remove(&m.task_id);
                    if let Some(task) = self.known_tasks.get_mut(&m.task_id) {
                        task.assigned_to = Some(m.winner_id);
                        task.status = TaskState::Assigned;
                    }
                    if m.winner_id == self.id && self.assigned_task.is_none() {
                        if let Some(task) = self.known_tasks.get(&m.task_id).cloned() {
                            self.assigned_task = Some((task.task_id, task.pickup, task.dropoff));
                            self.state = RobotState::Planning {
                                task_id: task.task_id,
                                pickup: task.pickup,
                                dropoff: task.dropoff,
                            };
                        }
                    }
                }
            }
        }

        // 2b. Check peer timeouts (lost peer detection > 5 ticks)
        let mut dead_peers = Vec::new();
        for (&peer, &last_tick) in &self.last_heartbeats {
            if self.current_tick > last_tick + 5 {
                dead_peers.push(peer);
            }
        }

        let mut tasks_to_reauction = Vec::new();
        for peer in dead_peers {
            self.local_reservations.remove_peer(peer);
            self.local_wfg.remove_robot(peer);
            self.last_heartbeats.remove(&peer);

            for task in self.known_tasks.values_mut() {
                if task.assigned_to == Some(peer) && task.status == TaskState::InProgress {
                    task.status = TaskState::Reassigned;
                    task.assigned_to = None;
                    tasks_to_reauction.push(task.clone());
                }
            }
        }

        for task in tasks_to_reauction {
            self.send(FleetMessage::TaskStatus(task.clone()));
            self.send(FleetMessage::AuctionOpen(AuctionOpenMsg {
                task_id: task.task_id,
                pickup: task.pickup,
                dropoff: task.dropoff,
                deadline_tick: self.current_tick + self.auction_config.bid_window_ticks,
            }));
        }

        // 2c. If Idle and unassigned, check for unassigned open tasks and open auctions
        if self.state == RobotState::Idle && self.assigned_task.is_none() {
            let unassigned_tasks: Vec<TaskStatusMsg> = self
                .known_tasks
                .values()
                .filter(|t| t.status == TaskState::Open || t.status == TaskState::Reassigned)
                .cloned()
                .collect();

            for task in unassigned_tasks {
                if !self.pending_auctions.contains_key(&task.task_id) {
                    let mut auction = Auction::new(task.task_id, task.pickup, task.dropoff, self.current_tick, 2);
                    let my_cost = compute_bid_cost(&self.auction_config, self.pos, self.battery, task.pickup, task.dropoff, 0.0, 0, Some(self.current_tick + 2), self.current_tick);
                    auction.add_bid(self.id, &BidMsg { task_id: task.task_id, cost: my_cost });
                    self.pending_auctions.insert(task.task_id, auction);

                    self.send(FleetMessage::AuctionOpen(AuctionOpenMsg {
                        task_id: task.task_id,
                        pickup: task.pickup,
                        dropoff: task.dropoff,
                        deadline_tick: self.current_tick + 2,
                    }));
                }
            }
        }

        // 2d. Close any open auctions
        let mut awards = Vec::new();
        let mut unawarded_expired = Vec::new();
        let mut already_assigned_winners: HashSet<RobotId> = HashSet::new();

        let mut auction_ids: Vec<TaskId> = self.pending_auctions.keys().copied().collect();
        auction_ids.sort();

        for task_id in auction_ids {
            if let Some(auction) = self.pending_auctions.get(&task_id) {
                if auction.is_closed(self.current_tick) {
                    let mut valid_bids = auction.bids.clone();
                    valid_bids.retain(|b| !already_assigned_winners.contains(&b.bidder_id));

                    if let Some(best) = valid_bids.iter().min_by(|a, b| {
                        a.cost
                            .partial_cmp(&b.cost)
                            .unwrap_or(std::cmp::Ordering::Equal)
                            .then_with(|| a.bidder_id.cmp(&b.bidder_id))
                    }) {
                        already_assigned_winners.insert(best.bidder_id);
                        awards.push(AwardMsg {
                            task_id,
                            winner_id: best.bidder_id,
                        });
                    } else {
                        unawarded_expired.push(task_id);
                    }
                }
            }
        }

        for task_id in unawarded_expired {
            self.pending_auctions.remove(&task_id);
        }

        for award in awards {
            self.pending_auctions.remove(&award.task_id);
            if let Some(task) = self.known_tasks.get_mut(&award.task_id) {
                task.status = TaskState::Assigned;
                task.assigned_to = Some(award.winner_id);
            }
            if let Some(task) = self.known_tasks.get(&award.task_id).cloned() {
                self.send(FleetMessage::TaskStatus(task));
            }
            if award.winner_id == self.id && self.assigned_task.is_none() {
                if let Some(task) = self.known_tasks.get(&award.task_id).cloned() {
                    self.assigned_task = Some((task.task_id, task.pickup, task.dropoff));
                    self.state = RobotState::Planning {
                        task_id: task.task_id,
                        pickup: task.pickup,
                        dropoff: task.dropoff,
                    };
                }
            }
            self.send(FleetMessage::Award(award));
        }

        // 2e. State machine execution
        match self.state.clone() {
            RobotState::Idle => {
                self.desired_next_pos = None;
            }
            RobotState::Bidding { .. } => {}
            RobotState::Planning { task_id, pickup, dropoff } => {
                let goal = if self.pos == pickup { dropoff } else { pickup };
                let mut planning_grid = (*self.grid).clone();
                for obs in &self.local_obstacles {
                    if planning_grid.in_bounds(*obs) {
                        planning_grid.set_cell(*obs, Cell::Wall);
                    }
                }

                let constraints = self.local_reservations.build_constraints_with_stationary(
                    &self.peer_poses,
                    self.current_tick,
                    100,
                );

                if let Some(path) = plan(
                    &planning_grid,
                    self.id,
                    self.pos,
                    self.current_tick,
                    goal,
                    &constraints,
                    200,
                ) {
                    self.current_intent_seq = self.next_seq;
                    self.current_intent_priority = self.id as u64;
                    self.local_reservations.reserve_own_path(&path);

                    self.send(FleetMessage::Intent(IntentMsg {
                        intent_seq: self.current_intent_seq,
                        path: path.clone(),
                        priority: self.current_intent_priority,
                    }));

                    if let Some(task) = self.known_tasks.get_mut(&task_id) {
                        task.status = TaskState::InProgress;
                    }
                    if let Some(task) = self.known_tasks.get(&task_id).cloned() {
                        self.send(FleetMessage::TaskStatus(task));
                    }

                    let conflicts = self.local_reservations.conflicts_with_peers(&path);
                    let mut must_yield = false;
                    for c in conflicts {
                        if should_yield(self.id, self.current_intent_priority, c.peer_id, c.peer_priority) {
                            must_yield = true;
                            self.send(FleetMessage::Yield(YieldMsg {
                                yielded_intent_seq: self.current_intent_seq,
                                to_robot: c.peer_id,
                                conflicting_cell: c.cell,
                                conflicting_tick: c.tick,
                            }));
                            break;
                        }
                    }

                    if must_yield {
                        self.state = RobotState::Replanning { task_id, pickup, dropoff };
                    } else {
                        self.state = RobotState::Moving { path, step_index: 0 };
                    }
                }
            }
            RobotState::Moving { path, step_index } => {
                if step_index + 1 >= path.len() {
                    // Reached intermediate or final goal
                    if let Some((task_id, pickup, dropoff)) = self.assigned_task {
                        if self.pos == pickup {
                            self.state = RobotState::Planning { task_id, pickup, dropoff };
                        } else if self.pos == dropoff {
                            self.state = RobotState::Idle;
                            self.assigned_task = None;
                            self.local_reservations.release_own();
                            if let Some(task) = self.known_tasks.get_mut(&task_id) {
                                task.status = TaskState::Completed;
                            }
                            if let Some(task) = self.known_tasks.get(&task_id).cloned() {
                                self.send(FleetMessage::TaskStatus(task));
                            }
                        }
                    } else {
                        self.state = RobotState::Idle;
                    }
                    self.desired_next_pos = None;
                } else {
                    let next_pos = path[step_index + 1].0;
                    let next_tick = self.current_tick + 1;

                    if self.local_obstacles.contains(&next_pos) {
                        if let Some((task_id, pickup, dropoff)) = self.assigned_task {
                            self.state = RobotState::Replanning { task_id, pickup, dropoff };
                        }
                        self.desired_next_pos = None;
                    } else {
                        // Check if any peer will occupy next_pos at next_tick
                        let mut next_pos_blocked = false;

                        for peer in self.local_reservations.all_peer_intents() {
                            let peer_at_next_pos = peer.path.iter().any(|&(p, t)| p == next_pos && t == next_tick);
                            if peer_at_next_pos {
                                let peer_prev = peer.path.iter().find(|&&(_, t)| t == self.current_tick).map(|&(p, _)| p);
                                if peer_prev == Some(next_pos) {
                                    // Peer was already sitting at next_pos -> physical occupancy blocks entry
                                    next_pos_blocked = true;
                                    break;
                                } else {
                                    // Both robots entering next_pos from outside at next_tick -> priority decides
                                    if should_yield(self.id, self.current_intent_priority, peer.robot_id, peer.priority) {
                                        next_pos_blocked = true;
                                        break;
                                    }
                                }
                            }

                            // Edge swap check: peer moving from next_pos to self.pos at next_tick
                            let peer_swapping = peer.path.windows(2).any(|w| {
                                let (from_pos, from_t) = w[0];
                                let (to_pos, to_t) = w[1];
                                from_t == self.current_tick && to_t == next_tick && from_pos == next_pos && to_pos == self.pos
                            });
                            if peer_swapping && should_yield(self.id, self.current_intent_priority, peer.robot_id, peer.priority) {
                                next_pos_blocked = true;
                                break;
                            }
                        }

                        // Also check latest known physical poses
                        let occupied_now = self.peer_poses.iter().any(|(&p_id, &(p_pos, _))| {
                            p_id != self.id && p_pos == next_pos
                        });

                        if next_pos_blocked || occupied_now {
                            if let Some((task_id, pickup, dropoff)) = self.assigned_task {
                                self.state = RobotState::Replanning { task_id, pickup, dropoff };
                            }
                            self.desired_next_pos = None;
                        } else {
                            let remaining: Vec<(Pos, Tick)> = path[step_index + 1..]
                                .iter()
                                .enumerate()
                                .map(|(offset, (pos, _))| (*pos, next_tick + offset as u64))
                                .collect();

                            let conflicts = self.local_reservations.conflicts_with_peers(&remaining);
                            if !conflicts.is_empty() {
                                let c = &conflicts[0];
                                if should_yield(self.id, self.current_intent_priority, c.peer_id, c.peer_priority) {
                                    self.local_wfg.add_wait(self.id, c.peer_id);
                                    let deadlocks = resolve_deadlocks(&self.local_wfg);
                                    if deadlocks.contains(&self.id) {
                                        if let Some((task_id, pickup, dropoff)) = self.assigned_task {
                                            self.state = RobotState::Replanning { task_id, pickup, dropoff };
                                        }
                                    }
                                    self.desired_next_pos = None;
                                } else {
                                    self.send(FleetMessage::Conflict(ConflictMsg {
                                        challenger_intent_seq: self.current_intent_seq,
                                        challenger_priority: self.current_intent_priority,
                                        challenged_id: c.peer_id,
                                        challenged_intent_seq: c.peer_intent_seq,
                                        conflicting_cell: c.cell,
                                        conflicting_tick: c.tick,
                                    }));
                                    self.desired_next_pos = Some(next_pos);
                                }
                            } else {
                                self.desired_next_pos = Some(next_pos);
                            }
                        }
                    }
                }
            }
            RobotState::Yielding { .. } => {
                if let Some((task_id, pickup, dropoff)) = self.assigned_task {
                    self.state = RobotState::Replanning { task_id, pickup, dropoff };
                }
                self.desired_next_pos = None;
            }
            RobotState::Replanning { task_id, pickup, dropoff } => {
                self.local_reservations.release_own();
                self.state = RobotState::Planning { task_id, pickup, dropoff };
                self.desired_next_pos = None;
            }
            RobotState::Dead => {
                self.alive = false;
                self.desired_next_pos = None;
            }
        }

        // 2f. Periodic heartbeat & pose broadcast
        let status = match self.state {
            RobotState::Idle => RobotStatus::Idle,
            RobotState::Bidding { .. } => RobotStatus::Idle,
            RobotState::Planning { .. } => RobotStatus::Planning,
            RobotState::Moving { .. } => RobotStatus::Moving,
            RobotState::Yielding { .. } => RobotStatus::Yielding,
            RobotState::Replanning { .. } => RobotStatus::Planning,
            RobotState::Dead => RobotStatus::Dead,
        };

        self.send(FleetMessage::Heartbeat(HeartbeatMsg {
            tick: self.current_tick,
            battery: self.battery,
        }));
        self.send(FleetMessage::Pose(PoseMsg {
            pos: self.pos,
            tick: self.current_tick,
            battery: self.battery,
            status,
        }));

        // 2g. Battery consumption
        self.battery = (self.battery - 0.0005).max(0.0);
        if self.battery <= 0.0 {
            self.state = RobotState::Dead;
            self.alive = false;
        }

        self.update_telemetry(status);
    }

    fn handle_detected_conflict(&mut self, conflict: &crate::planner::PeerConflict) {
        if should_yield(self.id, self.current_intent_priority, conflict.peer_id, conflict.peer_priority) {
            self.send(FleetMessage::Yield(YieldMsg {
                yielded_intent_seq: self.current_intent_seq,
                to_robot: conflict.peer_id,
                conflicting_cell: conflict.cell,
                conflicting_tick: conflict.tick,
            }));
            if let Some((task_id, pickup, dropoff)) = self.assigned_task {
                self.state = RobotState::Replanning { task_id, pickup, dropoff };
            }
        } else {
            self.send(FleetMessage::Conflict(ConflictMsg {
                challenger_intent_seq: self.current_intent_seq,
                challenger_priority: self.current_intent_priority,
                challenged_id: conflict.peer_id,
                challenged_intent_seq: conflict.peer_intent_seq,
                conflicting_cell: conflict.cell,
                conflicting_tick: conflict.tick,
            }));
        }
    }

    /// Phase 4: Atomic simultaneous movement commit.
    pub fn commit_movement(&mut self) -> (Pos, Option<Pos>) {
        let prev = self.pos;
        if let Some(next) = self.desired_next_pos.take() {
            self.pos = next;
            if let RobotState::Moving { ref mut step_index, .. } = self.state {
                *step_index += 1;
            }
            (prev, Some(next))
        } else {
            (prev, None)
        }
    }

    fn update_telemetry(&self, status: RobotStatus) {
        let path = match self.state {
            RobotState::Moving { ref path, step_index } => {
                path[step_index..].iter().map(|(p, _)| *p).collect()
            }
            _ => Vec::new(),
        };
        let task = self.assigned_task.map(|(t, _, _)| t);
        let _ = self.telemetry_tx.send(RobotTelemetry {
            id: self.id,
            pos: self.pos,
            tick: self.current_tick,
            battery: self.battery,
            status,
            path,
            task,
        });
    }
}
