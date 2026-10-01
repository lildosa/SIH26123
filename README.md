# THADAM: Trajectory-Aware Heuristics for Autonomous Decentralized AMR Mesh

> **Edge-Native P2P Robot Fleet Coordination Engine**
> **SIH 2026 Problem Statement — Bharat Electronics Limited (BEL)**
> **Stack:** Rust 1.85+ (2024 Edition) | WebGL (Three.js) | ONNX Runtime / Pure-Rust Inference
> **Highlights:** 100% ISO 3691-4 Zero-Collision Guarantee | Zero Single Point of Failure | 48 KB Embedded Neural Guide | 204 ns Adaptive Bidding

---

## Executive Summary

Centralized AMR fleet managers suffer from three major vulnerabilities: **Single Points of Failure (SPOF)**, **network saturation**, and **stale global state** during dynamic obstacle recovery.

**THADAM** solves this with a fully decentralized, edge-native peer-to-peer (P2P) mesh engine written in Rust. Every AMR acts as an autonomous actor running **Space-Time A*** pathfinding, **Contract Net (CNP)** task auctions, and **Wait-For-Graph (WFG)** deadlock resolution on local compute—communicating via asynchronous UDP Multicast without any central server.

---

## Key Feature: Embedded Edge-AI Layer (Post-PPT Innovation)

> **Post-PPT Innovation:** Developed after initial SIH submission, THADAM integrates two pure-Rust, zero-cloud Edge-AI components. Both are strictly **advisory**: all ISO 3691-4 safety invariants are deterministically enforced with AI enabled or disabled (76/76 unit/integration tests pass; 0 collisions across 360 benchmarked runs).

* **Live Interactive Web Console:** [thadam.up.railway.app](https://thadam.up.railway.app/)
* **Model Provenance & Weights:** [huggingface.co/sanjeevafk/thadam-guidance-fcn](https://huggingface.co/sanjeevafk/thadam-guidance-fcn)

| Feature | Neural A* Guidance (Navigation) | LinUCB Adaptive Bidding (Task Allocation) |
| --- | --- | --- |
| **Architecture** | 48,161-parameter Dilated FCN (~48 KB FP32 / 194 KB ONNX) embedded in binary | Contextual Bandit ($d=6$ features, 4 weight profiles) per robot |
| **Role** | Predicts detour heatmaps over a 32×32 window to guide A* open set | Learns optimal bid weightings from auction win/loss context |
| **Performance** | < 0.5 ms inference per planning call | **204 ns** per bid decision, 66 ns per update |
| **Hard Safety** | Reverts to pure kinematic A* if search exceeds **800 expansions** | Modifies bid values only; auction arbitration and awards remain deterministic |
| **Impact** | 32×32 grid (8 AMRs): Unguided A* completes 0/10 tasks; Guided completes **8/10** | 15×15 grid (6 AMRs): Task completion increases from **82% to 94%** |

```
               +-------------------------------------------------+
               |             DECENTRALIZED AMR NODE              |
               |                                                 |
               |   +------------------+   +------------------+   |
               |   | Space-Time A*    |   | Contract Net     |   |
               |   | (Neural-Guided)  |   | (LinUCB Bidding) |   |
               |   +--------+---------+   +--------+---------+   |
               |            |                      |             |
               |   +--------+----------------------+---------+   |
               |   |      Wait-For-Graph Cycle Breaker       |   |
               |   +-------------------+---------------------+   |
               |                       |                         |
               |   +-------------------+---------------------+   |
               |   |    P2P UDP Multicast Mesh (239.0.26.123)|   |
               |   +-------------------+---------------------+   |
               |                       |                         |
               |   +-------------------+---------------------+   |
               |   |  ISO 3691-4 Safety HAL & Sensor Watchdog |  |
               |   +-----------------------------------------+   |
               +-------------------------------------------------+

```

---

## Core Technical Innovations

1. **Space-Time Grid with Heading Latency:** Pathfinding across $(x, y, \theta, t)$ explicitly models rotational delay (90° = 1 tick, 180° = 2 ticks). AMRs lock space-time cells during turns to eliminate directional edge-swap collisions.
2. **Contract Net Protocol (CNP) Auctions:** Multi-factor bidding evaluating travel distance, local congestion, battery levels, delay risks, and deadlines.
3. **Distributed WFG Deadlock Resolution:** Transitive dependency graphs detect cycle bottlenecks ($R_1 \to R_2 \to R_3 \to R_1$) in $O(V+E)$ time using gossip messages. Priority rules force the lowest-priority node to yield and re-route.
4. **Resilient Network Transport:** Utilizes Lamport logical clocks for causal event ordering, UDP sequence deduplication, dual-burst Forward Error Correction (FEC) for control frames, and single-parity XOR blocks for bulk transfers.

---

## Benchmark Comparison (15×15 Grid)

*Evaluated against a Centralized Conflict-Based Search (CBS) dispatcher running FIFO batch pathfinding:*

| Fleet Scale | Distributed Makespan | Centralized CBS Makespan | Throughput Ratio (Dist / CBS) | Safety Pass Rate |
| --- | --- | --- | --- | --- |
| **2 AMRs** | 97 ticks | 59 ticks | 0.61× | 100% (0 Collisions) |
| **4 AMRs** | 62 ticks | 38 ticks | 0.61× | 100% (0 Collisions) |
| **6 AMRs** | 60 ticks | 23 ticks | 0.38× | 100% (0 Collisions) |

> **Trade-Off Analysis:** Decentralized coordination accepts higher makespan latency in small grids to gain **zero single point of failure**, **local dynamic re-planning**, and **mesh network fault tolerance**. On large/congested grids (32×32), Neural Guidance re-establishes high throughput where static heuristics fail.

---

## Quick Start & Local Development

### 1. Launch Local Web Console (3D Digital Twin)

```bash
make start
# Opens local 3D Digital Twin & Live Control Dashboard at http://localhost:3000

```

### 2. Headless Simulation with Edge-AI

```bash
cd engine
cargo run --release -- sim --robots 8 --width 32 --height 32 --tasks 10 \
  --neural-guidance --learned-bids --seed 3

```

### 3. Verification & Benchmarks

```bash
make test   # Execute all 76 automated integration tests
make bench  # Run comparative benchmark against Centralized CBS

```

---

## Team Innovation Igniters

Ashish S | Sanjeev kumar S | Kamlesh Y | Prajan SS | Sangamithra B | Sudhishna P

**Developed for SIH 2026** | Bharat Electronics Limited (BEL) Problem Statement 26123