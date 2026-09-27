pub mod bandit;
pub mod guidance;

pub use bandit::{
    BANDIT_ARMS, BANDIT_DIM, LinUcbBandit, arm_weights, bid_context, completion_reward,
};
pub use guidance::{GUIDANCE_GRID, GuidanceEngine, SharedGuidance, heatmap_cost_at};
