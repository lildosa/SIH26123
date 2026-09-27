//! Measures neural guidance inference latency (one heatmap per planning call).
//! Run with: cargo run --release --example guidance_latency
use sih26123::ai::GuidanceEngine;
use sih26123::world::{GridMap, Pos};
use std::time::Instant;

fn main() {
    let mut engine = GuidanceEngine::load().expect("embedded model must compile");
    let grid = GridMap::generate_warehouse(32, 32, 3);
    let goal = Pos::new(30, 30);

    // Warm-up (allocations, caches).
    for _ in 0..20 {
        let _ = engine.infer_heatmap(&grid, goal).unwrap();
    }

    const N: usize = 2000;
    let t0 = Instant::now();
    let mut sink = 0.0f32;
    for i in 0..N {
        let g = Pos::new(i % 32, (i * 7) % 32);
        let heat = engine.infer_heatmap(&grid, g).unwrap();
        sink += heat[0];
    }
    let dt = t0.elapsed();
    println!(
        "infer_heatmap: {:.1} us/call over {} calls on 32x32 warehouse (sink={})",
        dt.as_nanos() as f64 / 1000.0 / N as f64,
        N,
        sink
    );
}
