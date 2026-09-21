use crate::protocol::TaskId;
use crate::world::Pos;
use crate::protocol::Tick;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RobotState {
    Idle,
    Bidding { task_id: TaskId },
    Planning { task_id: TaskId, pickup: Pos, dropoff: Pos },
    Moving { path: Vec<(Pos, Tick)>, step_index: usize },
    Yielding { resume_after_replan: bool },
    Replanning { task_id: TaskId, pickup: Pos, dropoff: Pos },
    Dead,
}
