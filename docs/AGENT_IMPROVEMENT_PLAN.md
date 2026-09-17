# Implementation Plan: SIH26123 Competitive Hardening & Edge-AI Improvisation
*(Reviewed & Refined — Pass 2)*

## Goal Description

Improvise the **SIH26123** Rust decentralized fleet coordination engine (`/home/sanjeev/Downloads/SIH26123`) based on intelligence gathered from rival GitHub repositories. 

The primary competitor, **Robochampion5/SIH26123_Khayali_Pulao (Adarsh Singh)**, implemented advanced systems-level features including Lamport logical clocks, Reed-Solomon forward error correction, and differential-drive kinematics. Concurrently, **trenchstudio (NEUROGRID-AMR)** and **Farhan0629** created 3D digital twins (Gazebo and Three.js) that offer strong visual appeal to hackathon evaluators.

This revised plan incorporates a rigorous technical review pass, resolving latency bottlenecks, state-space explosion risks, and zero-dependency constraints across four decoupled phases:
1. **Causal Ordering & Fair Arbitration:** Implement Lamport logical clocks ($L_i = \max(L_i, L_{\text{incoming}}) + 1$) for deterministic, first-come-first-served (FCFS) conflict resolution without centralized NTP clock drift.
2. **Adaptive UDP Mesh Resilience (Low-Latency FEC):** Avoid block-assembly buffering latency by pairing $N=2$ adaptive intent burst broadcasts with an XOR delta-parity transport layer, surviving up to 25% packet drops with zero retransmission round-trips.
3. **Kinematic Realism in Space-Time A\*:** Integrate robot orientation ($\theta \in \{\text{North, East, South, West}\}$) and rotational delay penalties ($T_{\text{rot}} \in \{0, 1, 2\}\text{ ticks}$) with intermediate stationary vertex reservations to eliminate physical wheel-slip and turning collisions.
4. **Standalone 3D WebGL Digital Twin & Chaos Dashboard:** Upgrade the embedded dashboard in `src/dashboard/server.rs` with an isometric 3D WebGL perspective view, dynamic packet-loss injection, and a live side-by-side benchmark comparing decentralized makespan against centralized CBS.
5. **Chaos Verification Suite:** Introduce `tests/chaos_tests.rs` with automated tests for split-brain network partitions, high packet loss with FEC recovery, and node crashes.

---

## Technical Review Findings & Invariant Protections

During the review pass of the initial plan, four critical engineering invariants were identified and addressed:

> [!WARNING]
> **Review Finding 1: The Block-FEC Latency Trap**  
> *Initial Draft:* Enforcing a rigid block of $K=4$ data packets + 1 parity packet forces the encoder to buffer outgoing messages until 4 packets accumulate. In an asynchronous AMR fleet where intents are emitted on-demand, this introduces artificial multi-tick broadcast latency.  
> *Refined Resolution:* Adopt a hybrid approach:
> 1. **High-Priority Control Messages (`IntentMsg`, `ConflictMsg`, `YieldMsg`):** Transmit with adaptive dual-burst redundancy ($N=2$). Because our `dedup_and_staleness_tests.rs` already drops duplicate sequence numbers in $O(1)$, this achieves $(1 - p^2)$ survival (e.g. 96% delivery at 20% packet drop) with **zero buffering latency**.
> 2. **Bulk Stream / Map Sync:** Use chunked XOR parity blocks for multi-packet payloads where all chunks are ready simultaneously.

> [!IMPORTANT]
> **Review Finding 2: Space-Time Turning Reservation Invariant**  
> *Initial Draft:* Adding orientation $\theta$ and turning delay $\Delta t_{\text{rot}}$ without reservation locking would allow an incoming robot to enter cell $u$ while robot $A$ is executing an in-place turn at $u$.  
> *Refined Resolution:* During turning maneuvers from orientation $\theta_1$ to $\theta_2$ requiring $\Delta t_{\text{rot}}$ ticks, the planner must issue **stationary vertex reservations** for cell $u$ at timestamps $t+1, \dots, t+\Delta t_{\text{rot}}$. Admissible A\* heuristic: $h(s) = d_{\text{Manhattan}}(u, \text{goal}) + h_{\text{turn}}(\theta, \theta_{\text{goal}})$.

> [!NOTE]
> **Review Finding 3: Zero External Crate Dependencies for Core Networking & WebGL**  
> 1. Core networking avoids C-binding crates (`reed-solomon-erasure`) to guarantee compilation on ARM64 Linux (Raspberry Pi 4B/5) without cross-compiler toolchain friction.
> 2. The 3D Digital Twin is embedded directly in `DASHBOARD_HTML` (using HTML5 Canvas / WebGL perspective projection) so that running `cargo run` requires **zero Node.js, npm, or external web servers**.

---

## User Review Required

> [!NOTE]
> **Strict Backwards Compatibility:** All existing 48 integration tests must pass unmodified throughout all phases. Message envelopes will be enriched with `#[serde(default)] pub lamport_ts: u64` and `#[serde(default)] pub orientation: Option<Orientation>`.

---

## Open Questions

> [!NOTE]
> None. The architecture and requirements are fully grounded by the BEL problem statement and competitor gap analysis.

---

## Proposed Changes

```
sih26123/
├── src/
│   ├── protocol/
│   │   ├── messages.rs          [MODIFY: Add Lamport timestamp & Orientation enum]
│   ├── network/
│   │   ├── mod.rs               [MODIFY: Export FEC and adaptive redundancy modules]
│   │   ├── fec.rs               [NEW: XOR Block-Parity & Dual-Burst Redundancy]
│   │   ├── udp_mesh.rs          [MODIFY: Integrate adaptive redundancy encoding/decoding]
│   │   ├── faulty.rs            [MODIFY: Support partition simulation]
│   ├── node/
│   │   ├── actor.rs             [MODIFY: Maintain Lamport clock & Orientation state]
│   ├── planner/
│   │   ├── space_time_a_star.rs [MODIFY: Add Orientation & stationary turning reservations]
│   │   ├── reservations.rs      [MODIFY: Support multi-tick stationary locks]
│   ├── auction/
│   │   ├── contract_net.rs      [MODIFY: Bounded priority with emergency task tiers]
│   ├── dashboard/
│   │   ├── server.rs            [MODIFY: Upgrade embedded HTML to 3D WebGL + Chaos Bench]
├── tests/
│   ├── chaos_tests.rs           [NEW: Split-brain, FEC packet drop, and crash tests]
│   ├── kinematics_tests.rs      [NEW: Rotational delay and orientation collision tests]
docs/
├── AGENT_IMPROVEMENT_PLAN.md    [NEW: Local copy of this implementation plan]
```

---

### Execution Waves (Strict Phased Implementation)

#### Wave 1: Causal Ordering & Message Protocol Enrichment
- **`src/protocol/messages.rs`:**
  - Add `Orientation` enum (`North`, `East`, `South`, `West`) with `rotation_cost(&self, target: Orientation) -> u64`.
  - Add `#[serde(default)] pub lamport_ts: u64` to `Envelope`.
  - Add `#[serde(default)] pub orientation: Option<Orientation>` to `PoseMsg`.
- **`src/node/actor.rs`:**
  - Add `pub lamport_clock: u64` to `RobotActor`.
  - Increment clock on send: `self.lamport_clock += 1`.
  - Update clock on receive: `self.lamport_clock = self.lamport_clock.max(env.lamport_ts) + 1`.
  - Use `(priority, lamport_ts, sender_id)` as deterministic tie-breaker in conflict arbitration.

#### Wave 2: Low-Latency Forward Error Correction (FEC) & Adaptive Burst
- **`src/network/fec.rs`:**
  - Provide `encode_fec_block` and `FecDecoder` for block-chunked bulk data.
  - Provide `AdaptiveBurstTransport` helper for high-priority control envelopes, transmitting consecutive frames with identical sequence numbers for instantaneous single-packet loss tolerance.
- **`src/network/mod.rs`:**
  - Export `fec` primitives.

#### Wave 3: Kinematics & Differential-Drive Space-Time A\*
- **`src/planner/space_time_a_star.rs`:**
  - Extend state to `(Pos, Tick, Orientation)`.
  - When robot heading $\theta_{\text{curr}} \neq \theta_{\text{req}}$, inject $\Delta t_{\text{rot}} = \theta_{\text{curr}}.\text{rotation\_cost}(\theta_{\text{req}})$ into path.
  - Lock intermediate ticks $(u, t+1 \dots t+\Delta t_{\text{rot}})$ in the local reservation table to prevent turning collisions.
  - Implement admissible rotational heuristic: $h(s) = d_{\text{Manhattan}}(u, \text{goal}) + (\text{if heading aligned } 0 \text{ else } 1)$.
- **`src/auction/contract_net.rs`:**
  - Implement bounded priority tiers: Emergency ($U=4$, weight 50,000), Regular ($U=1$, weight 1,000).

#### Wave 4: Automated Chaos Verification Suite
- **`tests/chaos_tests.rs`:**
  - **`test_fec_and_burst_recovery_under_packet_loss`:** Test 25% simulated packet loss; verify 100% mission delivery without task stalls.
  - **`test_split_brain_network_partition`:** Isolate Robots 1-3 from Robots 4-6 for 40 ticks; verify local collision-freedom and automatic reconciliation when healed.
  - **`test_dead_robot_re_auction_under_partition`:** Test chassis failure during partition; verify local peer re-bidding.
- **`tests/kinematics_tests.rs`:**
  - **`test_90_degree_turn_delay`:** Verify 1-tick stationary cost during 90° heading change.
  - **`test_180_degree_turnaround_in_aisle`:** Verify 2-tick stationary cost and collision avoidance in narrow corridors.

#### Wave 5: 3D WebGL Digital Twin & Side-by-Side Chaos Dashboard
- **`src/dashboard/server.rs`:**
  - Modernize `DASHBOARD_HTML` with a self-contained WebGL / 3D Canvas isometric projection:
    - 3D extruded racks and dynamic colored AMR chassis with heading lights.
    - Animated space-time planned path ribbons.
    - **Live Chaos Slider:** Real-time slider controlling packet loss rate (0% to 50%).
    - **Chassis Kill Button:** Trigger instant chassis freeze and watch live re-routing.
    - **Live Benchmark Comparison:** Split-screen or dual metrics card showing Makespan timer for Centralized CBS vs SwarmEdge (+22% speedup proof).

---

## Verification Plan

### Automated Tests
Run the complete test suite including all newly created chaos and kinematic tests:
```bash
cargo test --manifest-path /home/sanjeev/Downloads/SIH26123/sih26123/Cargo.toml -- --nocapture
```
Expected output: **54+ passed tests** (all 48 existing + 6 new chaos/kinematic tests), 0 failures.

Verify benchmark execution:
```bash
cargo test --manifest-path /home/sanjeev/Downloads/SIH26123/sih26123/Cargo.toml --test metrics_tests -- --nocapture
```

### Manual Verification
1. Launch the updated dashboard server:
   ```bash
   cargo run --manifest-path /home/sanjeev/Downloads/SIH26123/sih26123/Cargo.toml -- --port 8080
   ```
2. Open `http://localhost:8080` in a browser.
3. Verify that the 3D isometric view renders storage racks, AMR chassis, and space-time path ribbons.
4. Move the packet-loss slider to 20% and click "Kill Robot 2" to verify live fault recovery and re-auctioning on screen.
