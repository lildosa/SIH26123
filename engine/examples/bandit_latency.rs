//! Measures LinUCB bid-decision latency (choose_arm + weight application).
//! Run with: cargo run --release --example bandit_latency
use sih26123::ai::{LinUcbBandit, bid_context};
use std::time::Instant;

fn main() {
    let mut bandit = LinUcbBandit::new();
    // Warm the exploration schedule so the measured path is steady-state UCB.
    let ctx = bid_context(0.4, 0.9, 1, 0.2, Some(30), 6);
    for _ in 0..100 {
        let arm = bandit.choose_arm(&ctx);
        bandit.update(arm, &ctx, 0.5);
    }

    // Steady-state decision latency over 100k samples.
    const N: usize = 100_000;
    let t0 = Instant::now();
    let mut sink = 0usize;
    for i in 0..N {
        let mut b = bandit.clone();
        let c = bid_context(
            (i % 100) as f64 / 100.0,
            0.5 + (i % 50) as f32 / 100.0,
            i % 3,
            (i % 25) as f64 / 100.0,
            Some(10 + (i % 40) as u64),
            1 + i % 8,
        );
        sink += b.choose_arm(&c);
    }
    let dt = t0.elapsed();
    println!(
        "choose_arm: {:.0} ns/call over {} calls (sink={})",
        dt.as_nanos() as f64 / N as f64,
        N,
        sink % 7
    );

    // Update latency (closed-form rank-1).
    let t1 = Instant::now();
    for i in 0..N {
        let c = bid_context(0.3, 0.9, 1, 0.1, Some(25), 4);
        bandit.update(i % 4, &c, 0.5);
    }
    let dt1 = t1.elapsed();
    println!(
        "update:     {:.0} ns/call over {} calls",
        dt1.as_nanos() as f64 / N as f64,
        N
    );
}
