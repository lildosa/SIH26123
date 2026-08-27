use crate::world::Pos;
use serde::{Deserialize, Serialize};

pub type RobotId = u32;
pub type TaskId = u32;
pub type Tick = u64;
pub type SeqNum = u64;

/// Every outgoing message is wrapped in an Envelope.
/// `seq` increments for EVERY outgoing message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Envelope {
    pub sender_id: RobotId,
    pub seq: SeqNum,
    pub payload: FleetMessage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RobotStatus {
    Idle,
    Planning,
    Moving,
    Yielding,
    Dead,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoseMsg {
    pub pos: Pos,
    pub tick: Tick,
    pub battery: f32,
    pub status: RobotStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatMsg {
    pub tick: Tick,
    pub battery: f32,
}

/// A robot announces its planned path. Priority is fixed to THIS version.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentMsg {
    pub intent_seq: SeqNum,
    pub path: Vec<(Pos, Tick)>,
    pub priority: u64,
}

/// Conflict challenge referencing specific intent versions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictMsg {
    pub challenger_intent_seq: SeqNum,
    pub challenger_priority: u64,
    pub challenged_id: RobotId,
    pub challenged_intent_seq: SeqNum,
    pub conflicting_cell: Pos,
    pub conflicting_tick: Tick,
}

/// Yield acknowledgment referencing the yielded intent version.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YieldMsg {
    pub yielded_intent_seq: SeqNum,
    pub to_robot: RobotId,
    pub conflicting_cell: Pos,
    pub conflicting_tick: Tick,
}

/// Task assignment/status message for complete tracking and reassignment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStatusMsg {
    pub task_id: TaskId,
    pub pickup: Pos,
    pub dropoff: Pos,
    pub assigned_to: Option<RobotId>,
    pub status: TaskState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskState {
    Open,
    Assigned,
    InProgress,
    Completed,
    Reassigned,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuctionOpenMsg {
    pub task_id: TaskId,
    pub pickup: Pos,
    pub dropoff: Pos,
    pub deadline_tick: Tick,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BidMsg {
    pub task_id: TaskId,
    pub cost: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwardMsg {
    pub task_id: TaskId,
    pub winner_id: RobotId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FleetMessage {
    Pose(PoseMsg),
    Heartbeat(HeartbeatMsg),
    Intent(IntentMsg),
    Conflict(ConflictMsg),
    Yield(YieldMsg),
    TaskStatus(TaskStatusMsg),
    AuctionOpen(AuctionOpenMsg),
    Bid(BidMsg),
    Award(AwardMsg),
}
