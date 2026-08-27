# SIH26123: Edge-AI Distributed AMR Fleet Coordination Engine

> **Smart India Hackathon 2026 Problem Statement**  
> **Organization:** Bharat Electronics Limited (BEL)  
> **Domain:** Robotics, Edge-AI, Autonomous Mobile Robots (AMRs), Defense & Industrial Logistics  
> **Language & Toolchain:** Rust 1.85+ / 2024 Edition  
> **Key Metric:** 100% Zero-Collision Guarantee (ISO 3691-4) and >= 20% Makespan Reduction vs Centralized Baselines  

---

## 1. Executive Summary

In high-density industrial and defense logistics warehouses, centralized fleet management systems suffer from three critical structural weaknesses:

1. **Single Point of Failure (SPOF):** If the central server, dispatch coordinator, or Wi-Fi access point fails, the entire robot fleet freezes.
2. **Network Bandwidth & Latency Saturation:** Centralized servers must ingest raw high-frequency telemetry from every AMR and compute global multi-agent paths, scaling exponentially ($O(N!)$ or $O(V^N)$).
3. **Stale Real-Time State:** Dynamic obstacles (fallen pallets, humans, broken robots) require round-trip server replanning, causing latency delays and hazardous physical bottlenecks.

**SIH26123** is a fully decentralized, edge-native peer-to-peer (P2P) AMR coordination engine built entirely in Rust. Every robot acts as an autonomous actor running Space-Time A* pathfinding, Contract Net task auctions, and Wait-For-Graph deadlock resolution on local edge compute, communicating directly over asynchronous UDP Multicast mesh networking without requiring any centralized master server.

---

## 2. System Architecture & Core Innovations

```
+-----------------------------------------------------------------------------------+
|                            DECENTRALIZED AMR ACTOR NODE                           |
|                                                                                   |
|  +---------------------------+  +--------------------------+  +----------------+  |
|  |     Space-Time A*         |  |   Contract Net Auction   |  | Wait-For-Graph |  |
|  |  Multi-Agent Pathfinding  |  |    Multi-Factor Bidding  |  | Cycle Breaker  |  |
|  +-------------+-------------+  +------------+-------------+  +-------+--------+  |
|                |                             |                        |           |
|  +-------------+-----------------------------+------------------------+--------+  |
|  |                 5-Phase Synchronous Edge State Machine                      |  |
|  |           [ Sense -> Decide -> Flush/Deliver -> Move -> Evaluate ]          |  |
|  +---------------------------------------+-------------------------------------+  |
|                                          |                                        |
|  +---------------------------------------+-------------------------------------+  |
|  |                       P2P Mesh Network Abstraction                          |  |
|  |      - In-Memory Tick-Scoped Bus (Simulation & Benchmarking)                |  |
|  |      - Fault Injection Transport (Drop Rate, Duplication, Latency)          |  |
|  |      - Real Asynchronous UDP Multicast (239.0.26.123:26123 for Hardware)    |  |
|  +---------------------------------------+-------------------------------------+  |
|                                          |                                        |
|  +---------------------------------------+-------------------------------------+  |
|  |                   ISO 3691-4 Fail-Safe Hardware HAL                         |  |
|  |      - Local LiDAR / Sonar Physical Sensing (< 50ms Emergency Brake)        |  |
|  |      - Serial Bridge to Microcontroller (Arduino Uno / STM32 via 115200)   |  |
|  +-----------------------------------------------------------------------------+  |
+-----------------------------------------------------------------------------------+
```

### 2.1 Space-Time A* Pathfinding (3D Reservation Grid)
- Path trajectories are planned in 3-dimensional space-time $(x, y, t)$.
- Enforces strict vertex non-occupancy constraints (preventing two robots on the same cell at tick $t$) and directional edge-swap constraints (preventing robots from crossing adjacent cells in opposite directions $(u, v) \leftrightarrow (v, u)$).
- When a robot's path is blocked, it negotiates or waits in-place using temporal wait moves before re-routing.

### 2.2 Contract Net Protocol (CNP) Multi-Factor Auctions
- When a warehouse order arrives, AMRs autonomously broadcast task announcements over the P2P mesh.
- Idle robots compute a multi-factor marginal cost function:
  $$\text{Cost} = w_{\text{travel}} \cdot d_{\text{travel}} + w_{\text{congestion}} \cdot C_{\text{pickup}} + w_{\text{battery}} \cdot (1 - B) + w_{\text{delay}} \cdot \text{Delay} + w_{\text{deadline}} \cdot \text{Penalty}$$
- Bids are collected within a configured tick window (default: 5 ticks), and the contract is awarded deterministically to the lowest-cost bidder with tie-breaking by Robot ID.

### 2.3 Wait-For-Graph (WFG) Deadlock Cycle Detection
- In narrow corridors and 4-way intersections, robots construct local dependency graphs where edge $R_i \to R_j$ indicates that $R_i$ is waiting for $R_j$ to vacate a cell.
- A depth-first search (DFS) algorithm detects circular wait dependencies ($R_1 \to R_2 \to R_3 \to R_1$) in $O(V + E)$ time.
- Cycles are resolved deterministically: the robot with the lowest priority yields and re-routes, breaking gridlocks without human intervention.

### 2.4 ISO 3691-4 Fail-Safe Sensing & Dead Chassis Handling
- Each robot continuously senses obstacles within a 3-cell radius.
- If a peer AMR breaks down or loses battery, its chassis remains stationary. Surviving peers detect missed heartbeats (5-tick timeout), treat the dead robot chassis as a permanent static obstacle in local maps, re-auction any incomplete tasks, and route around the broken chassis with zero collisions.

---

## 3. Quantitative Performance & Verification

The system includes a comparative benchmarking pipeline evaluating the decentralized engine against a centralized Conflict-Based Search (CBS) dispatcher with FIFO queuing.

```
+------------------------------------------------------------------------------------+
| SCALE       | DISTRIBUTED MAKESPAN | CENTRALIZED CBS MAKESPAN | THROUGHPUT SPEEDUP |
+-------------+----------------------+--------------------------+--------------------+
| 2 AMRs      | 28 ticks             | 36 ticks                 | + 22.2%            |
| 4 AMRs      | 44 ticks             | 58 ticks                 | + 24.1%            |
| 6 AMRs      | 62 ticks             | 86 ticks                 | + 27.9%            |
| 8 AMRs      | 79 ticks             | 118 ticks                | + 33.1%            |
+------------------------------------------------------------------------------------+
| SAFETY STATS: 0 Vertex Collisions | 0 Edge Swap Collisions | 100% Invariant Pass   |
+------------------------------------------------------------------------------------+
```

All 49 automated integration tests across 11 test suites pass with 100% reliability:
- `auction_tests` (6 tests): Idle bidding, congestion scaling, tie-breaking, deadline urgency.
- `baseline_tests` (2 tests): Centralized CBS pathfinder and FIFO dispatcher validation.
- `deadlock_tests` (9 tests): Simple cycle, 3-robot cycle, multiple independent cycles, priority yielding.
- `dedup_and_staleness_tests` (6 tests): Monotonic sequence ordering, duplicate rejection, stale view resilience.
- `full_validation` (1 test): Multi-scale speedup validation and zero-collision invariants.
- `metrics_tests` (2 tests): Telemetry collector aggregation and comparative metrics.
- `network_fault_tests` (4 tests): 100% packet loss, packet duplication, and staged latency delivery.
- `network_tests` (4 tests): Tick-scoped delivery and zero self-echo validation.
- `planner_tests` (9 tests): Space-Time A*, edge-swap conflict detection, bottleneck waiting.
- `scenario_tests` (3 tests): Choke-point navigation, dynamic obstacle replanning, peer kill reassignment.
- `simulation_tests` (3 tests): Synchronous 5-phase execution and zero collision multi-task runs.

---

## 4. Single-Command Startup & Quick Start

### 4.1 Using Make (Recommended)

To compile and launch the interactive Web Operations Console on port 3000:
```bash
make start
```

Other available Make targets:
```bash
make bench        # Run comparative benchmark against Centralized CBS
make sim          # Run headless simulation (4 AMRs, 8 tasks)
make test         # Execute all 49 automated integration tests
make build        # Compile release binary
make docker-up    # Launch containerized service via Docker Compose
make docker-down  # Stop Docker containers
make clean        # Clean build artifacts
```

### 4.2 Using Docker Compose

```bash
docker compose up --build -d
```
Access the web console at `http://localhost:3000`.

### 4.3 Using Cargo Directly

```bash
# Launch Web Dashboard
cargo run --release --bin sih26123 -- dashboard --port 3000 --robots 4 --tasks 8

# Run Benchmarks
cargo run --release --bin sih26123 -- bench --width 15 --height 15 --tasks 5

# Run Headless Simulation
cargo run --release --bin sih26123 -- sim --robots 8 --width 20 --height 20 --tasks 15
```

---

## 5. Web Operations Console Features

The passive web console (`http://localhost:3000`) provides real-time observational telemetry and interactive fleet controls built with Axum, WebSockets, and HTML5 Canvas:

- **Fleet Scale Selector:** Dynamically scale the fleet between 2, 4, 6, 8, and 10 AMRs in real-time with automatic path assignment and unique robot coloring.
- **Interactive Wall Tool:** Click any warehouse cell to inject or remove dynamic obstacles and observe instantaneous peer rerouting.
- **Custom Task Dispatcher:** Click two coordinates on the grid (Pickup $\to$ Dropoff) to inject a custom order into the live P2P auction pool.
- **Direct AMR Move Tool:** Select any AMR and click a destination cell to issue direct waypoint overrides.
- **Preset Test Scenarios:**
  1. *Head-On Bottleneck:* 2 AMRs crossing a 1-lane corridor.
  2. *4-Way Gridlock:* 4 AMRs crossing a 4-way intersection simultaneously.
  3. *Fleet Rush:* 8 concurrent orders distributed across the warehouse.
- **Node Kill / Restore Controls:** Toggle individual AMR failures (`Kill R1` / `Restore R1`) to demonstrate live fault tolerance and task re-allocation.
- **Speed Controller:** Live tick rate slider (20ms to 400ms per tick).

---

## 6. Hardware-in-the-Loop (HIL) Physical Deployment

The architecture supports mixed-reality Hardware-in-the-Loop (HIL) operation where 1 physical robot operates alongside $N$ virtual peer robots:

- **Raspberry Pi 4B/5:** Runs the `sih26123` Rust binary, acts as AMR-1, and communicates with virtual peers over UDP Multicast (`239.0.26.123:26123`).
- **Arduino Uno:** Connected to the Pi via USB Serial (`115200 8N1`), controlling an L298N motor driver, 2x DC motors, an HC-SR04 ultrasonic distance sensor, and status LEDs.
- **Hardware Failsafe Watchdog:** If serial communication between the Pi and Arduino drops for $> 500\text{ ms}$, the microcontroller firmware automatically cuts motor power.
- **Physical Sensor Overrides:** When an obstacle is detected within 15 cm by the physical ultrasonic sensor, the Arduino sends an immediate `OBS:<dist>` packet, causing the Space-Time planner on the Pi to halt the physical robot and replan around the obstacle.

Detailed hardware wiring pinouts, serial protocol definitions, and complete Arduino C++ firmware are available in [`docs/HARDWARE_INTEGRATION_PLAN.md`](docs/HARDWARE_INTEGRATION_PLAN.md).

---

## 7. Project Structure

```
SIH26123/
+-- Cargo.toml                    # Root Cargo workspace manifest
+-- Dockerfile                    # Multi-stage container build
+-- docker-compose.yml            # Docker Compose service definition
+-- Makefile                      # Single-command build and run workflows
+-- README.md                     # Comprehensive project documentation
+-- docs/                         # Specifications, plans, and architecture diagrams
|   +-- HARDWARE_INTEGRATION_PLAN.md # Exhaustive Raspberry Pi + Arduino Uno HIL plan
|   +-- IMPLEMENTATION_PLAN.md    # 9-phase software execution specification
|   +-- SIH_2026_Problem_Statements.pdf # Official BEL Problem Statement
|   +-- diagrams/                 # Architecture, flowchart, and roadmap diagrams
+-- scripts/
|   +-- demo_matplotlib.py        # Standalone Python Matplotlib animated visualizer
+-- sih26123/                     # Main Rust crate
    +-- Cargo.toml
    +-- src/
    |   +-- lib.rs                # Library exports
    |   +-- main.rs               # CLI entry point (sim, bench, dashboard)
    |   +-- world/                # GridMap, Pos, Cell representations
    |   +-- protocol/             # Envelopes, Heartbeats, Intents, Bids, Conflicts
    |   +-- planner/              # Space-Time A*, Reservation Table, Constraints
    |   +-- negotiator/           # Wait-For-Graph, Cycle Detection, Priority Arbitration
    |   +-- auction/              # Contract Net Protocol, Multi-Factor Bidding
    |   +-- network/              # InMemoryBus, FaultyNetwork, UdpMulticastTransport
    |   +-- node/                 # RobotActor, State Machine, Telemetry, Sensing
    |   +-- sim/                  # 5-Phase Synchronous Simulation Runner
    |   +-- baseline/             # Centralized Conflict-Based Search (CBS) baseline
    |   +-- metrics/              # Metrics Collector & Comparative Benchmark Engine
    |   +-- dashboard/            # Axum WebSocket server & Industrial Canvas Console
    +-- tests/                    # 11 integration test suites (49 passing tests)
        +-- auction_tests.rs
        +-- baseline_tests.rs
        +-- deadlock_tests.rs
        +-- dedup_and_staleness_tests.rs
        +-- full_validation.rs
        +-- metrics_tests.rs
        +-- network_fault_tests.rs
        +-- network_tests.rs
        +-- planner_tests.rs
        +-- scenario_tests.rs
        +-- simulation_tests.rs
```

---

## 8. License & Attribution

Developed for the **Smart India Hackathon (SIH) 2026** under the **Bharat Electronics Limited (BEL)** Problem Statement: *Edge-AI Based Distributed Fleet Coordination for Autonomous Mobile Robots (AMRs) in Smart Warehouses*.
