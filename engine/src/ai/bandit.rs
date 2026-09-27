//! LinUCB Contextual Bandit — learned bidding weights for the Contract Net
//! auction (Track 2, SIH26123 Edge-AI).
//!
//! Each robot maintains a disjoint-model LinUCB over `d = 6` context features.
//! The bandit does NOT learn "which robot should win" — it learns **how a
//! robot should weight its own cost features** when bidding, so that bids
//! made with a given weight profile lead to fast task completion.
//!
//! Arms are 4 discrete weight profiles over the same features used by
//! `compute_bid_cost` (the "conservative" arm reproduces the static
//! auction defaults exactly, so the learned policy starts at baseline
//! behavior and may only deviate when evidence accumulates).
//!
//! Reward per completed task: completion speed normalized into (0, 1]
//! (1.0 = no wait at all). LinUCB updates are closed-form; the whole
//! state is 4 arms × (6×6 matrix A + 6-vector b) ≈ 912 f32 = ~3.6 KB,
//! with zero heap allocation in the choose/update path.

use serde::{Deserialize, Serialize};

/// Context dimension.
pub const BANDIT_DIM: usize = 6;
/// Number of discrete weight-profile arms.
pub const BANDIT_ARMS: usize = 4;

pub type VecN = [f32; BANDIT_DIM];
pub type MatN = [[f32; BANDIT_DIM]; BANDIT_DIM];

/// Exploration coefficient. 0.3 gives moderate exploration early that
/// decays naturally as ||A^-1 x|| shrinks with evidence.
pub const ALPHA: f32 = 0.3;

/// Discrete bidding weight profiles (the arms). Index order = arm id.
///
/// Weights follow `AuctionConfig` field order:
/// [travel, congestion, battery, delay, deadline].
const ARMS: [[f64; 5]; BANDIT_ARMS] = [
    // 0: Conservative — identical to AuctionConfig::default().
    [1.0, 0.3, 0.5, 0.8, 0.4],
    // 1: Speed-first — travel dominates; congestion & delay discounted.
    [2.0, 0.15, 0.2, 0.4, 0.4],
    // 2: Load-balanced — backlog and battery matter more (spreads work).
    [1.0, 0.3, 1.0, 2.0, 0.4],
    // 3: Aggressive near-field — grabs close tasks, ignores own backlog.
    [1.5, 0.1, 0.1, 0.05, 0.2],
];

pub fn arm_weights(arm: usize) -> [f64; 5] {
    ARMS[arm.min(BANDIT_ARMS - 1)]
}

/// Build the d=6 context vector for a bid decision. All features are
/// normalized to roughly [0, 1] so the closed-form updates stay stable.
///
/// x = [norm_travel, battery_deficit, backlog, congestion, deadline_urgency, fleet_density]
pub fn bid_context(
    travel_norm: f64,
    battery: f32,
    backlog: usize,
    congestion: f64,
    deadline_ticks_left: Option<u64>,
    active_fleet: usize,
) -> VecN {
    let travel = travel_norm.clamp(0.0, 1.0) as f32;
    let battery_deficit = (1.0 - battery as f64).clamp(0.0, 1.0) as f32;
    let backlog_n = (backlog as f64 / 4.0).min(1.0) as f32;
    let congestion_n = congestion.clamp(0.0, 1.0) as f32;
    // Urgency: 1 = no deadline pressure, decays as the deadline approaches.
    let urgency = match deadline_ticks_left {
        Some(t) if t > 0 => (t as f32 / 50.0).min(1.0),
        Some(_) => 0.0,
        None => 1.0,
    };
    let fleet_n = (active_fleet as f64 / 8.0).min(1.0) as f32;
    [
        travel,
        battery_deficit,
        backlog_n,
        congestion_n,
        urgency,
        fleet_n,
    ]
}

/// Disjoint-model LinUCB bandit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinUcbBandit {
    /// Per-arm inverse covariance matrices A_a^-1 (identity-initialized).
    inv_a: [MatN; BANDIT_ARMS],
    /// Per-arm reward vectors b_a.
    b: [VecN; BANDIT_ARMS],
    /// Completed-task updates per arm (telemetry).
    pub arm_counts: [u32; BANDIT_ARMS],
    /// Bid selections per arm — drives exploration scheduling, because in
    /// an auction only the *winner* gets feedback (a losing bid produces
    /// no update, so update counts cannot gate exploration).
    pub selections: [u32; BANDIT_ARMS],
    /// Cumulative reward per arm (telemetry).
    pub arm_reward_sum: [f32; BANDIT_ARMS],
}

impl Default for LinUcbBandit {
    fn default() -> Self {
        Self::new()
    }
}

impl LinUcbBandit {
    pub fn new() -> Self {
        Self {
            inv_a: [[[0.0; BANDIT_DIM]; BANDIT_DIM]; BANDIT_ARMS],
            b: [[0.0; BANDIT_DIM]; BANDIT_ARMS],
            arm_counts: [0; BANDIT_ARMS],
            selections: [0; BANDIT_ARMS],
            arm_reward_sum: [0.0; BANDIT_ARMS],
        }
    }

    /// Identity-initialized inverse covariances (LinUCB warm start).
    fn init(&mut self) {
        for a in 0..BANDIT_ARMS {
            for i in 0..BANDIT_DIM {
                for j in 0..BANDIT_DIM {
                    self.inv_a[a][i][j] = if i == j { 1.0 } else { 0.0 };
                }
            }
        }
    }

    fn ensure_init(&mut self) {
        // inv_a is all-zero right after deserialization/const construction.
        if self.inv_a[0][0][0] == 0.0 {
            self.init();
        }
    }

    /// UCB scores per arm: θ_aᵀx + α·√(xᵀ A_a⁻¹ x).
    /// Returns None only if a matrix became singular (never in practice
    /// given the identity ridge; falls back to arm 0 in `choose_arm`).
    pub fn ucb_scores(&mut self, x: &VecN) -> Option<[f32; BANDIT_ARMS]> {
        self.ensure_init();
        let mut scores = [0.0f32; BANDIT_ARMS];
        for a in 0..BANDIT_ARMS {
            let theta = mat_vec(&self.inv_a[a], &self.b[a]);
            let mean = dot(&theta, x);
            let ax = mat_vec(&self.inv_a[a], x);
            let bonus = ALPHA * sqrt_axt_a(&ax, x);
            if !mean.is_finite() || !bonus.is_finite() {
                return None;
            }
            scores[a] = mean + bonus;
        }
        Some(scores)
    }

    /// Select the arm with the highest UCB score (ties → lowest arm id,
    /// which keeps arm 0 = static baseline in early exploration).
    ///
    /// UCB1-style warm start: every arm is *selected* (bid with) at least
    /// `WARM_START_PLAYS` times before UCB exploitation kicks in. Counted on
    /// selections, not reward updates — in an auction only the winner gets
    /// feedback, so update counts starve exploration.
    pub fn choose_arm(&mut self, x: &VecN) -> usize {
        const WARM_START_PLAYS: u32 = 2;
        let arm = if let Some(&min_count) = self.selections.iter().min() {
            if min_count < WARM_START_PLAYS {
                self.selections
                    .iter()
                    .enumerate()
                    .min_by_key(|&(a, &c)| (c, a))
                    .map(|(a, _)| a)
                    .unwrap_or(0)
            } else {
                match self.ucb_scores(x) {
                    Some(scores) => {
                        let mut best = 0usize;
                        let mut best_v = f32::NEG_INFINITY;
                        for (a, &v) in scores.iter().enumerate() {
                            if v > best_v {
                                best_v = v;
                                best = a;
                            }
                        }
                        best
                    }
                    None => 0,
                }
            }
        } else {
            0
        };
        self.selections[arm] = self.selections[arm].saturating_add(1);
        arm
    }

    /// Closed-form LinUCB update for the selected arm.
    /// A_a ← A_a + x xᵀ, b_a ← b_a + r·x (we keep A⁻¹ incrementally).
    pub fn update(&mut self, arm: usize, x: &VecN, reward: f32) {
        self.ensure_init();
        let a = arm.min(BANDIT_ARMS - 1);

        // ax = A⁻¹ x (needed before A⁻¹ is rank-updated).
        let ax = mat_vec(&self.inv_a[a], x);
        let xta = dot(x, &ax);
        if xta <= 1e-12 {
            return; // degenerate context; skip update
        }

        // Rank-1 downdate of the inverse: A⁻¹ ← A⁻¹ − (A⁻¹x)(A⁻¹x)ᵀ / (1 + xᵀA⁻¹x)
        let denom = 1.0 + xta;
        for i in 0..BANDIT_DIM {
            for j in 0..BANDIT_DIM {
                self.inv_a[a][i][j] -= ax[i] * ax[j] / denom;
            }
        }

        // b ← b + r·x
        for i in 0..BANDIT_DIM {
            self.b[a][i] += reward * x[i];
        }

        self.arm_counts[a] = self.arm_counts[a].saturating_add(1);
        self.arm_reward_sum[a] += reward;
    }

    /// Index of the most-selected arm (the learned "policy so far").
    pub fn best_arm(&self) -> usize {
        let mut best = 0usize;
        let mut best_c = self.arm_counts[0];
        for (a, &c) in self.arm_counts.iter().enumerate() {
            if c > best_c {
                best_c = c;
                best = a;
            }
        }
        best
    }
}

#[inline]
fn dot(a: &[f32; BANDIT_DIM], b: &[f32; BANDIT_DIM]) -> f32 {
    let mut s = 0.0;
    for i in 0..BANDIT_DIM {
        s += a[i] * b[i];
    }
    s
}

#[inline]
fn mat_vec(m: &MatN, v: &VecN) -> VecN {
    let mut out = [0.0f32; BANDIT_DIM];
    for i in 0..BANDIT_DIM {
        let mut s = 0.0;
        for j in 0..BANDIT_DIM {
            s += m[i][j] * v[j];
        }
        out[i] = s;
    }
    out
}

/// √(xᵀ (A⁻¹x)) — the exploration bonus magnitude.
#[inline]
fn sqrt_axt_a(ax: &VecN, x: &VecN) -> f32 {
    let v = dot(x, ax).max(0.0);
    v.sqrt()
}

/// Reward shaping: completion speed → (0, 1]. 1.0 = instant, decays with
/// the pickup+dropoff travel length actually taken relative to the
/// theoretical Manhattan minimum. `actual_ticks` is when the robot finished
/// relative to when it was assigned; `min_travel` is the Manhattan lower
/// bound for the task's full journey at assignment time.
pub fn completion_reward(actual_ticks: u64, min_travel: u64) -> f32 {
    if actual_ticks == 0 {
        return 1.0;
    }
    let ratio = min_travel.max(1) as f32 / actual_ticks as f32;
    ratio.clamp(0.05, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn learns_preferred_arm_from_consistent_rewards() {
        let mut b = LinUcbBandit::new();
        let good = bid_context(0.2, 1.0, 0, 0.0, None, 2);
        let _bad = bid_context(0.9, 0.2, 4, 1.0, Some(5), 8);

        // Arm 2 consistently pays off for this context; others don't fire.
        for _ in 0..200 {
            let arm = b.choose_arm(&good);
            let reward = if arm == 2 { 0.9 } else { 0.1 };
            b.update(arm, &good, reward);
        }
        let scores = b.ucb_scores(&good).unwrap();
        assert!(
            scores[2] > scores[0] && scores[2] > scores[1] && scores[2] > scores[3],
            "arm 2 should dominate after consistent rewards: {scores:?}"
        );
        assert_eq!(b.best_arm(), 2);
    }

    #[test]
    fn conservative_arm_matches_static_defaults() {
        let w = arm_weights(0);
        assert_eq!(w, [1.0, 0.3, 0.5, 0.8, 0.4]);
    }

    #[test]
    fn reward_bounds_respected() {
        assert_eq!(completion_reward(0, 10), 1.0);
        let r = completion_reward(100, 10);
        assert!((0.05..=1.0).contains(&r));
        assert_eq!(completion_reward(50, 50), 1.0);
    }

    #[test]
    fn update_is_stable_under_repeated_identical_contexts() {
        let mut b = LinUcbBandit::new();
        let x = bid_context(0.5, 0.5, 1, 0.5, Some(20), 4);
        for _ in 0..500 {
            let arm = b.choose_arm(&x);
            b.update(arm, &x, 0.5);
        }
        // No NaNs after many rank-1 updates.
        for a in 0..BANDIT_ARMS {
            for row in &b.inv_a[a] {
                for &v in row {
                    assert!(v.is_finite());
                }
            }
        }
    }
}
