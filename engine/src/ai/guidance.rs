//! Neural A* Guidance — pure-Rust ONNX inference via tract.
//!
//! A 5-layer fully-convolutional network (48,161 parameters, ~48 KB fp32)
//! predicts a 2D scalar cost-to-go heatmap over a fixed 32×32 grid window.
//! The planner uses it as an *advisory* heuristic that reorders BinaryHeap
//! open-set priorities; hard vertex/edge reservations and the 800-expansion
//! safety fallback remain 100% enforced in `space_time_a_star.rs` (ISO 3691-4).
//!
//! Inference cadence: one forward pass per planning call (not per node).
//! `GuidanceEngine::infer_heatmap` fills the reusable input buffer, runs the
//! compiled static-shape plan, and returns the 32×32 heatmap; the A* loop
//! then performs O(1) `f32` lookups per successor — no allocation in the
//! hot search loop.

use crate::world::{GridMap, Pos};
use std::path::Path;
use std::sync::OnceLock;

/// Fixed ONNX tensor side — training and export use this exact shape.
/// Grids larger than 32 are cropped from the window origin; smaller grids
/// are zero-padded (channel 0 = 0 there, so predictions are ignored).
pub const GUIDANCE_GRID: usize = 32;

const CH: usize = GUIDANCE_GRID * GUIDANCE_GRID; // 1024 cells per channel

const MODEL_BYTES: &[u8] = include_bytes!("../../assets/guidance.onnx");

/// Runnable static-shape plan (compiled once at load, reused every call).
/// `into_runnable` hands back an `Arc<SimplePlan>`; `run` lives on the
/// `Runnable` trait (implemented for the `Arc` via the runtime layer).
pub type GuidanceRunnable = std::sync::Arc<tract_onnx::prelude::TypedRunnableModel>;

/// Loaded guidance model + reusable input buffer.
pub struct GuidanceEngine {
    plan: GuidanceRunnable,
    input: tract_onnx::prelude::Tensor,
}

impl GuidanceEngine {
    /// Compile the compile-time-embedded ONNX graph.
    pub fn load() -> Result<Self, String> {
        Self::from_bytes(MODEL_BYTES)
    }

    /// Load and compile the guidance model from an ONNX file on disk.
    pub fn from_path(path: &Path) -> Result<Self, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("read {:?}: {e}", path))?;
        Self::from_bytes(&bytes)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        use tract_onnx::prelude::*;

        let model = tract_onnx::onnx()
            .model_for_read(&mut std::io::Cursor::new(bytes))
            .map_err(|e| format!("parse onnx: {e}"))?
            .with_input_fact(
                0,
                InferenceFact::dt_shape(f32::datum_type(), &[1, 4, GUIDANCE_GRID, GUIDANCE_GRID]),
            )
            .map_err(|e| format!("input fact: {e}"))?
            .into_typed()
            .map_err(|e| format!("typing: {e}"))?
            .into_optimized()
            .map_err(|e| format!("optimize: {e}"))?
            .into_runnable()
            .map_err(|e| format!("compile plan: {e}"))?;

        let input = Tensor::zero::<f32>(&[1, 4, GUIDANCE_GRID, GUIDANCE_GRID])
            .map_err(|e| format!("alloc input: {e}"))?;

        Ok(Self { plan: model, input })
    }

    /// Run one forward pass; returns the flat 32×32 cost-to-go heatmap.
    ///
    /// Channel layout (must mirror scripts/train_guidance_model.py):
    ///   ch0 = walkable (1 free / 0 wall), ch1 = (x − goal.x)/32,
    ///   ch2 = (y − goal.y)/32, ch3 = manhattan(goal)/64.
    ///
    /// `grid` is sampled with wrap-free clamped indexing when it is smaller
    /// than the 32×32 window (padded cells read as walkable = 0).
    pub fn infer_heatmap(&mut self, grid: &GridMap, goal: Pos) -> Option<Vec<f32>> {
        use tract_onnx::prelude::*;

        let goal_f = |v: usize, g: usize| (v as f32 - g as f32) / GUIDANCE_GRID as f32;

        {
            let mut view = self.input.view_mut();
            let data = view.as_slice_mut::<f32>().ok()?;

            for y in 0..GUIDANCE_GRID {
                let gy = y.min(grid.height - 1);
                for x in 0..GUIDANCE_GRID {
                    let gx = x.min(grid.width - 1);
                    let base = y * GUIDANCE_GRID + x;
                    if x < grid.width && y < grid.height {
                        let walk = if grid.is_walkable(Pos::new(gx, gy)) {
                            1.0f32
                        } else {
                            0.0
                        };
                        data[base] = walk;
                        data[CH + base] = goal_f(x, goal.x);
                        data[2 * CH + base] = goal_f(y, goal.y);
                    } else {
                        data[base] = 0.0;
                        data[CH + base] = 0.0;
                        data[2 * CH + base] = 0.0;
                    }
                    let manhattan = x.abs_diff(goal.x) + y.abs_diff(goal.y);
                    data[3 * CH + base] = manhattan as f32 / 64.0;
                }
            }
        }

        let outputs = self
            .plan
            .run(tvec!(self.input.clone().into_tvalue()))
            .map_err(|e| tracing::debug!("guidance inference failed: {e}"))
            .ok()?;

        let value = outputs.into_iter().next()?;
        let tensor = value.into_tensor();
        let heat = tensor.view().as_slice::<f32>().ok()?.to_vec();
        Some(heat)
    }
}

/// Advisory cost-to-go lookup from a heatmap. Clamped to ≥ 0, floored to
/// whole movement steps so it integrates with integer g-costs.
#[inline]
pub fn heatmap_cost_at(heat: &[f32], pos: Pos) -> usize {
    let v = heat[pos.y * GUIDANCE_GRID + pos.x];
    (v.max(0.0) * GUIDANCE_GRID as f32) as usize
}

/// Process-global shared engine. Robots clone the handle; the interior is
/// serialized behind a Mutex (one < 0.5 ms pass per planning call, tick
/// budget is 120 ms).
#[derive(Clone)]
pub struct SharedGuidance {
    inner: std::sync::Arc<std::sync::Mutex<GuidanceEngine>>,
}

impl SharedGuidance {
    pub fn new(engine: GuidanceEngine) -> Self {
        Self {
            inner: std::sync::Arc::new(std::sync::Mutex::new(engine)),
        }
    }

    /// Shared handle to the compile-time-embedded model. `None` when the
    /// embedded asset fails to parse/compile (guidance stays off).
    pub fn global() -> Option<Self> {
        static GLOBAL: OnceLock<Option<SharedGuidance>> = OnceLock::new();
        GLOBAL
            .get_or_init(|| match GuidanceEngine::load() {
                Ok(e) => {
                    tracing::info!("neural guidance model loaded (48,161 params)");
                    Some(SharedGuidance::new(e))
                }
                Err(err) => {
                    tracing::warn!("neural guidance unavailable: {err}");
                    None
                }
            })
            .clone()
    }

    /// One inference per planning call. `None` = inference failed; the
    /// planner must fall back to the pure kinematic heuristic.
    pub fn infer_heatmap(&self, grid: &GridMap, goal: Pos) -> Option<Vec<f32>> {
        let mut guard = self.inner.lock().ok()?;
        guard.infer_heatmap(grid, goal)
    }
}
