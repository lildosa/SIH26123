# THADAM: Trajectory-aware Heuristics for Autonomous Decentralized AMR Mesh

> **Distributed Edge-AI AMR Fleet Coordination Engine**  
> **Origin Context:** Smart India Hackathon (SIH) 2026 Problem Statement — Bharat Electronics Limited (BEL)  
> **Domain:** Robotics, Edge-AI, Autonomous Mobile Robots (AMRs), Defense & Industrial Logistics  
> **Language & Toolchain:** Rust 1.85+ / 2024 Edition  
> **Key Metric:** 100% Zero-Collision Guarantee (ISO 3691-4) and >= 20% Makespan Reduction vs Centralized Baselines  

---

## 1. Executive Summary

In high-density industrial and defense logistics warehouses, centralized fleet management systems suffer from three critical structural weaknesses:

1. **Single Point of Failure (SPOF):** If the central server, dispatch coordinator, or Wi-Fi access point fails, the entire robot fleet freezes.
2. **Network Bandwidth & Latency Saturation:** Centralized servers must ingest raw high-frequency telemetry from every AMR and compute global multi-agent paths, scaling exponentially ($O(N!)$ or $O(V^N)$).
3. **Stale Real-Time State:** Dynamic obstacles (fallen pallets, humans, broken robots) require round-trip server replanning, causing latency delays and hazardous physical bottlenecks.

**THADAM** (**T**rajectory-aware **H**euristics for **A**utonomous **D**ecentralized **A**MR **M**esh) is a fully decentralized, edge-native peer-to-peer (P2P) AMR coordination engine built entirely in Rust. Every robot acts as an autonomous actor running Space-Time A* pathfinding, Contract Net task auctions, and Wait-For-Graph deadlock resolution on local edge compute, communicating directly over asynchronous UDP Multicast mesh networking without requiring any centralized master server.

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

### 2.1 Discrete Space-Time Reservation Grid with Heading Change Latency
- Path trajectories are planned in 3-dimensional space-time $(x, y, t)$ with explicit robot heading: $s = (x, y, \theta, t)$ where $\theta \in \{\text{North}, \text{East}, \text{South}, \text{West}\}$.
- **Turn-Delay Cost Matrix:** Rotational latency is explicitly modeled as a discrete cost penalty rather than unphysical continuous curves: a 90° heading rotation costs a 1-tick delay, while a 180° turnaround costs 2 ticks. Kinematic heuristics (`kinematic_heuristic`) guide A* with rotational overheads.
- **Stationary Reservation Locks & Edge-Swap Verification:** While changing heading, the AMR locks its current cell in space-time across the turn duration, preventing incoming peers from encroaching. Directional edge-swap constraints ($(u, v) \leftrightarrow (v, u)$) are evaluated at the exact physical departure tick $(t + \Delta t_{\text{turn}})$, mathematically eliminating turning and crossing collisions.
- When a robot's path is blocked, it negotiates or waits in-place using temporal wait moves before re-routing.

### 2.2 Contract Net Protocol (CNP) Multi-Factor Auctions
- When a warehouse order arrives, AMRs autonomously broadcast task announcements over the P2P mesh.
- Idle robots compute a multi-factor marginal cost function:
  $$\text{Cost} = w_{\text{travel}} \cdot d_{\text{travel}} + w_{\text{congestion}} \cdot C_{\text{pickup}} + w_{\text{battery}} \cdot (1 - B) + w_{\text{delay}} \cdot \text{Delay} + w_{\text{deadline}} \cdot \text{Penalty}$$
- Bids are collected within a configured tick window (default: 5 ticks), and the contract is awarded deterministically by the authoritative designated auctioneer to the lowest-cost bidder with tie-breaking by Robot ID. Busy robots reject redundant awards, immediately re-opening tasks for peer bidding.

### 2.3 Distributed Wait-For-Graph (WFG) Gossip & Deadlock Cycle Breaking
- In narrow corridors and 4-way intersections, robots construct local dependency graphs where edge $R_i \to R_j$ indicates that $R_i$ is waiting for $R_j$ to vacate a cell.
- **Distributed WaitEdge Gossip:** When an AMR yields under conflict, it broadcasts `WaitEdgeMsg { waiter_id, blocking_id, tick, active: true }`. Peer AMRs ingest these messages into their local graphs, enabling transitive global cycle detection without any centralized coordinator.
- A depth-first search (DFS) algorithm detects circular wait dependencies ($R_1 \to R_2 \to R_3 \to R_1$) in $O(V + E)$ time.
- Cycles are resolved deterministically: the robot in the cycle with the lowest priority yields and re-routes, then broadcasts `WaitEdgeMsg { active: false }` to purge resolved edges across the mesh.

### 2.4 ISO 3691-4 Fail-Safe Sensing & Dead Chassis Handling
- Each robot continuously senses obstacles within a 3-cell radius.
- If a peer AMR breaks down or loses battery, its chassis remains stationary. Surviving peers detect missed heartbeats (5-tick timeout), treat the dead robot chassis as a permanent static obstacle in local maps, re-auction any incomplete tasks, and route around the broken chassis with zero collisions.

### 2.5 Causal Lamport Logical Clocks & Forward Error Correction (FEC)
- **Lamport Logical Clocks:** Every P2P broadcast embeds a monotonically increasing Lamport timestamp ($L$). Receivers update $L_{\text{local}} = \max(L_{\text{local}}, L_{\text{msg}}) + 1$. Deterministic arbitration orders concurrent claims by: (1) higher priority, (2) older Lamport timestamp, and (3) lower Robot ID.
- **UDP Sequence Deduplication:** Monotonic sequence numbers tracked per peer ($O(1)$ filter) immediately reject stale, out-of-order, or duplicated network packets.
- **Adaptive Dual-Burst FEC & Single-Parity XOR Blocks:** High-priority control frames (`Intent`, `Conflict`, `Yield`) use dual-burst transmission ($N=2$), ensuring survival probability $(1 - p^2)$ at drop rate $p$ (e.g. 96% delivery at 20% loss) with zero buffering latency. Bulk state transfers utilize systematic single-parity XOR block encoding.
- **Production Multicast Socket SO_REUSEPORT:** `UdpMeshNetwork` creates sockets configured with `SO_REUSEPORT` and `SO_REUSEADDR` via `socket2`, allowing multiple AMR processes to concurrently bind to port `26123` on Linux without socket contention.

---

## 3. Quantitative Performance & Verification

The system includes a comparative benchmarking pipeline evaluating the decentralized engine against a centralized Conflict-Based Search (CBS) dispatcher running concurrent multi-agent batch pathfinding with FIFO queuing.

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

All 61 automated integration tests across 14 test suites (plus unit tests) pass with 100% reliability:
- `auction_tests` (6 tests): Idle bidding, congestion scaling, tie-breaking, deadline urgency.
- `baseline_tests` (2 tests): Centralized concurrent multi-agent CBS pathfinder and FIFO dispatcher validation.
- `chaos_tests` (3 tests): Dual-burst FEC recovery, network partition split-brain, dead robot re-auction.
- `deadlock_tests` (9 tests): Simple cycle, 3-robot cycle, multiple independent cycles, priority yielding.
- `dedup_and_staleness_tests` (6 tests): Monotonic sequence ordering, duplicate rejection, stale view resilience.
- `full_validation` (1 test): Multi-scale speedup validation and zero-collision invariants.
- `kinematics_tests` (2 tests): 90° turn-delay cost, 180° aisle turnaround, stationary reservation locks.
- `lamport_tests` (14 tests): Causal ordering, Lamport clock increment/merge, deterministic tie-breaking.
- `metrics_tests` (2 tests): Telemetry collector aggregation and comparative metrics.
- `network_fault_tests` (4 tests): 100% packet loss, packet duplication, and staged latency delivery.
- `network_tests` (5 tests): Tick-scoped delivery, zero self-echo validation, and SO_REUSEPORT multicast socket sharing.
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
cd engine && cargo run --release -- dashboard --port 3000 --robots 4 --tasks 8

# Run Benchmarks
cd engine && cargo run --release -- bench --width 15 --height 15 --tasks 5

# Run Headless Simulation
cd engine && cargo run --release -- sim --robots 8 --width 20 --height 20 --tasks 15
```

---

## 5. Web Operations Console Features

The passive web console (`http://localhost:3000`) provides real-time observational telemetry and interactive fleet controls built with Axum, WebSockets, Three.js WebGL, and HTML5 Canvas:

- **Three.js WebGL 3D Digital Twin:** Real-time 3D rendered warehouse digital twin with industrial lighting, extruded metal shelving units, rotating LiDAR pucks, differential drive wheel animation, floating battery SoC % badges, and glowing 3D Space-Time trajectory ribbons. Supports intuitive mouse orbit controls (click-drag to orbit, wheel to zoom) and fallback to 2D isometric canvas.
- **Interactive Blocked Aisle Toggle (`🚨 Block Aisle`):** One-click button that dynamically injects obstacle blocks across primary warehouse corridors, forcing active AMRs to sense obstructions in real time and compute zero-collision reroutes.
- **Fleet Scale Selector:** Dynamically scale the fleet between 2, 4, 6, 8, and 10 AMRs in real-time with automatic path assignment and unique robot coloring.
- **Interactive Wall Tool:** Click any warehouse cell in 2D or 3D to inject or remove dynamic obstacles and observe instantaneous peer rerouting.
- **Custom Task Dispatcher:** Click two coordinates on the grid (Pickup $\to$ Dropoff) to inject a custom order into the live P2P auction pool.
- **Direct AMR Move Tool:** Select any AMR and click a destination cell to issue direct waypoint overrides.
- **Preset Test Scenarios:**
  1. *Head-On Bottleneck:* 2 AMRs crossing a 1-lane corridor.
  2. *4-Way Gridlock:* 4 AMRs crossing a 4-way intersection simultaneously.
  3. *Fleet Rush:* 8 concurrent orders distributed across the warehouse.
  4. *Blocked Aisle Corridor:* 4 AMRs in high-density corridors executing dynamic rerouting around a central obstruction.
- **Chaos Bench & Fault Injection:** Live packet loss slider (0% to 50%) demonstrating dual-burst ($N=2$) and XOR parity resilience, plus individual AMR kill/restore buttons.
- **Speed Controller:** Live tick rate slider (20ms to 400ms per tick).

---

## 6. Hardware-in-the-Loop (HIL) Physical Deployment

The architecture supports mixed-reality Hardware-in-the-Loop (HIL) operation where 1 physical robot operates alongside $N$ virtual peer robots:

- **Raspberry Pi 4B/5:** Runs the coordination engine binary, acts as AMR-1, and communicates with virtual peers over UDP Multicast (`239.0.26.123:26123`).
- **Arduino Uno:** Connected to the Pi via USB Serial (`115200 8N1`), controlling an L298N motor driver, 2x DC motors, an HC-SR04 ultrasonic distance sensor, and status LEDs.
- **Physical HIL Verification Script (`scripts/hil_serial_mock.py`):** Standalone zero-dependency Python verification mock that simulates 10 Hz ultrasonic telemetry with physical Gaussian sensor jitter ($\sigma = 1.2\text{ cm}$), threshold triggers (`OBS:11.4`), command echo handling, and the 500ms safety watchdog timer.
  ```bash
  # Run automated 10-test HIL loopback verification
  python3 scripts/hil_serial_mock.py --mode test

  # Spawn virtual Linux serial port (/tmp/ttyHIL_AMR) for external terminal connection
  python3 scripts/hil_serial_mock.py --mode pty
  ```
- **Hardware Failsafe Watchdog:** If serial communication between the Pi and Arduino drops for $> 500\text{ ms}$, the microcontroller firmware automatically cuts motor power.
- **Physical Sensor Overrides:** When an obstacle is detected within 15 cm by the physical ultrasonic sensor, the Arduino sends an immediate `OBS:<dist>` packet, causing the Space-Time planner on the Pi to halt the physical robot and replan around the obstacle.

Detailed hardware wiring pinouts, serial protocol definitions, and architecture specifications are available in [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md#5-hardware-in-the-loop-hil-integration-architecture).

---

## 7. Team Innovation Igniters

* Ashish S
* Sanjeev kumar S
* Kamlesh Y
* Prajan SS
* Sangamithra B
* Sudhishna P

---

## 8. License & Attribution

Developed for the **Smart India Hackathon (SIH) 2026** by Team **Innovation Igniters**. under the **Bharat Electronics Limited (BEL)** Problem Statement (26123): *Edge-AI Based Distributed Fleet Coordination for Autonomous Mobile Robots (AMRs) in Smart Warehouses*.
