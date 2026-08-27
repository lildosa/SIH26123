use crate::protocol::{AwardMsg, BidMsg, RobotId, TaskId, Tick};
use crate::world::Pos;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuctionConfig {
    pub w_travel: f64,     // travel time weight (default 1.0)
    pub w_congestion: f64, // congestion weight (default 0.3)
    pub w_battery: f64,    // battery drain weight (default 0.5)
    pub w_delay: f64,      // current-task delay penalty (default 0.8)
    pub w_deadline: f64,   // deadline urgency weight (default 0.4)
    pub bid_window_ticks: Tick,  // ticks to collect bids (default 5)
}

impl Default for AuctionConfig {
    fn default() -> Self {
        Self {
            w_travel: 1.0,
            w_congestion: 0.3,
            w_battery: 0.5,
            w_delay: 0.8,
            w_deadline: 0.4,
            bid_window_ticks: 5,
        }
    }
}

/// Compute bid cost. Lower cost = more desirable bid.
pub fn compute_bid_cost(
    config: &AuctionConfig,
    robot_pos: Pos,
    robot_battery: f32,
    pickup: Pos,
    dropoff: Pos,
    congestion_at_pickup: f64,
    current_task_remaining: usize,
    task_deadline: Option<Tick>,
    current_tick: Tick,
) -> f64 {
    let travel_time = (robot_pos.manhattan_distance(&pickup)
        + pickup.manhattan_distance(&dropoff)) as f64;
    let battery_cost = (1.0 - robot_battery as f64).max(0.0);
    let delay_cost = current_task_remaining as f64;
    let deadline_penalty = match task_deadline {
        Some(dl) if dl > current_tick => 1.0 / (dl - current_tick) as f64,
        Some(_) => 10.0,
        None => 0.0,
    };

    config.w_travel * travel_time
        + config.w_congestion * congestion_at_pickup * travel_time
        + config.w_battery * battery_cost
        + config.w_delay * delay_cost
        + config.w_deadline * deadline_penalty
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BidRecord {
    pub bidder_id: RobotId,
    pub cost: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Auction {
    pub task_id: TaskId,
    pub pickup: Pos,
    pub dropoff: Pos,
    pub opened_at: Tick,
    pub deadline: Tick,
    pub bids: Vec<BidRecord>,
}

impl Auction {
    pub fn new(
        task_id: TaskId,
        pickup: Pos,
        dropoff: Pos,
        opened_at: Tick,
        bid_window_ticks: Tick,
    ) -> Self {
        Self {
            task_id,
            pickup,
            dropoff,
            opened_at,
            deadline: opened_at + bid_window_ticks,
            bids: Vec::new(),
        }
    }

    pub fn add_bid(&mut self, bidder_id: RobotId, bid: &BidMsg) {
        if bid.task_id == self.task_id {
            // Replace existing bid from same robot if any, or insert new
            if let Some(existing) = self.bids.iter_mut().find(|b| b.bidder_id == bidder_id) {
                existing.cost = bid.cost;
            } else {
                self.bids.push(BidRecord {
                    bidder_id,
                    cost: bid.cost,
                });
            }
        }
    }

    pub fn is_closed(&self, current_tick: Tick) -> bool {
        current_tick >= self.deadline
    }

    /// Determines the winner: lowest cost wins; ties are broken deterministically by lowest RobotId.
    pub fn determine_winner(&self) -> Option<AwardMsg> {
        if self.bids.is_empty() {
            return None;
        }

        let best_bid = self.bids.iter().min_by(|a, b| {
            a.cost
                .partial_cmp(&b.cost)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.bidder_id.cmp(&b.bidder_id))
        })?;

        Some(AwardMsg {
            task_id: self.task_id,
            winner_id: best_bid.bidder_id,
        })
    }
}
