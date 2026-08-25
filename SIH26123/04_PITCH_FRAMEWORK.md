# SIH26123 — Pitch Framework: Impact · Innovation · Feasibility · Data · Scalability

> Problem-first, tech-later. Each section starts from the pain, then maps our solution onto it.

---

## 1. IMPACT — *"Who bleeds today, and how much?"*

**Problem:** Every AMR warehouse in India runs on an umbilical cord — a central server. Cut it (Wi-Fi dead zone between steel racks, server crash, network switch failure) and every robot freezes mid-aisle. Orders stall, SLAs break, workers idle.

**Quantified pain:**

| Dimension | Number | Source |
|---|---|---|
| India warehousing market | **$66.8B (2025) → $181B (2035)**, 10.5% CAGR | Expert Market Research — https://www.expertmarketresearch.com/reports/india-warehousing-market |
| India AMR market specifically | **$92.6M (2026) → $245M (2031)**, 17.6% CAGR — faster than global | MarketsandMarkets — https://www.marketsandmarkets.com/Market-Reports/geography/autonomous-mobile-robots-market/india |
| Global AMR market | $3.1B (2025) → $17B (2035), ~19.5% CAGR; **logistics/warehousing = 33.8%**, the #1 vertical | GM Insights — https://www.gminsights.com/industry-analysis/autonomous-mobile-robots-market |

**Impact chain (say this to judges):**

- **Operational:** one dead server = whole fleet down; decentralized fleet degrades *gracefully* — kill any robot, work continues (demo scenario 3 is literally this)
- **Economic:** ≥20% throughput gain on existing fleets = same robots, more orders, zero capex
- **Strategic (BEL angle):** Atmanirbhar Bharat — indigenous coordination layer breaks foreign FMS lock-in (OTTO/MiR/Geek+); critical for defense depots where foreign cloud dependency is unacceptable
- **Safety:** zero-collision guarantee protects goods and human co-workers

**One-liner:** *"We make warehouse uptime a property of the robots themselves — not of a server room."*

---

## 2. INNOVATION — *"What's genuinely new here?"*

**Problem:** The world has either (a) centralized products that fail at the center, or (b) papers that never left simulation. Nobody ships the middle.

| # | Innovation | Why it's not incremental |
|---|---|---|
| 1 | **Coordination moves from infrastructure into the robot** | Fleet intelligence becomes portable — works with ANY robot that speaks our protocol, like TCP/IP did for computers |
| 2 | **Dashboard-as-pure-listener architecture** | Inverts the industry pattern (VDA5050 standardizes hub-and-spoke; we eliminate the hub) |
| 3 | **Graceful degradation by auction** — robot death triggers automatic task re-bidding via heartbeats | Central systems can't do this; their task queue dies with the server |
| 4 | **Self-proving system**: baseline stop-and-wait mode is a toggle in the same binary, metrics live on screen | Judges don't believe claims; they believe side-by-side timers showing +20% |
| 5 | **Edge-first constraint as feature**: runs on ₹5k Raspberry Pi, not GPU cloud | Cost per robot ≈ negligible; deployable in dead-zone-heavy old warehouses |

**Framing line:** *"We didn't invent new math — we engineered the first integrated, metric-proven decentralization stack. Integration IS the innovation the industry failed to ship."*

---

## 3. FEASIBILITY — *"Can 6 students actually build this?"*

**Problem:** Most teams will propose "AI-powered robot swarm" vaporware with no runnable proof. Our feasibility comes from scope surgery.

| Risk factor | De-risk |
|---|---|
| Algorithms unproven? | All textbook-mature: A* (1968), D* Lite (2002), reservation tables (Silver 2005), Contract Net (1980) — decades of reference implementations |
| Hardware unavailable? | Nothing exotic: laptops simulate robot nodes now; Pi deployment is a stretch goal, not a dependency |
| Comms complexity? | ZeroMQ PUB/SUB + UDP discovery = ~200 lines, well-documented |
| Time? | PLAN.md Gantt: ~36h across 4 parallel tracks — core loop demoable by hour 12 |
| Success criteria vague? | PS defines them numerically: 0 collisions, +20% vs baseline. We measure live, no interpretation needed |
| Scope creep? | Hard cuts pre-agreed: CBS stretch-only; dashboard read-only; sim > real hardware priority |

**Kill-shot slide:** *"At hour 12 of development we already had 3 robots crossing one intersection with zero collisions"* — feasibility demonstrated, not argued.

---

## 4. DATA — *"Where does truth come from?"*

**Problem:** This PS needs no proprietary dataset — but judges still ask, so answer with rigor:

| Data need | Source | Status |
|---|---|---|
| Warehouse layouts | Procedurally generated grid worlds + published Kiva-style layouts from MAPF literature | Generated |
| Benchmark maps (credibility) | Sturtevant's MovingAI pathfinding benchmarks (standard in MAPF/CBS research) | Public — https://www.movingai.com/Benchmarks/ |
| Robot dynamics | Literature parameters: AMR speed ~1.0–1.5 m/s, accel limits per ISO 3691-4 operating zones | Calibrated constants |
| Live performance data | Self-generated: collisions, makespan, intersection wait time, reroutes, auction overhead — logged per run | Produced by system itself |
| Comparison baseline | Stop-and-wait mode, identical binary, identical scenario seeds → fair A/B | Controlled experiment |

**Judge-ready sentence:** *"Every number on our dashboard is reproducible: same seed, same scenario, two modes, one command."*

---

## 5. SCALABILITY — *"Does it survive success?"*

**Problem:** Centralized fleets hit a wall — add robots, server load and radio congestion grow until the coordinator chokes. Does ours?

**Technical scaling story:**

- **Communication scales with neighbors, not fleet size** — a robot negotiates only with robots sharing its aisle segment. O(local) vs central O(fleet). This is the architectural win
- **No ceiling component:** add robot = join mesh, nothing else changes. No server resize, no per-seat license
- **Demo proof:** run the same scenario at 3, 10, 25 robots → plot makespan & message-load curve live. Even if curves bend up at high density, they don't bend because of a coordinator — honest and defensible

**Honest limits (state them before judges find them):**

- Radio bandwidth contention at very high density (>~30 nodes/channel) → mitigation: channel segmentation / hierarchical clustering = future roadmap, not hackathon scope
- Very large maps → space-time A* memory grows; mitigated by windowed planning (already standard practice)

**Scaling beyond one warehouse:**

- Multi-floor/multi-zone: zones form federated meshes — natural extension of the peer model
- Interop path: VDA5050 gateway adapter so existing compliant robots join our mesh → adoption without rip-and-replace
- Business scale: same software spans a 10-robot dark store → 500-robot fulfillment center; India AMR market growing 17.6%/yr gives the runway

---

## The 60-Second Narrative (memorize)

> *"India's warehouses are automating fast — a $67B market running on robots tethered to a single server. When that server or its Wi-Fi hiccups, entire fleets freeze; when vendors lock you in, you pay forever. We asked: what if coordination lived inside the robots? Three robots that gossip, negotiate intersections, auction tasks among themselves, and survive each other's deaths — proven live against a stop-and-wait baseline with zero collisions and 20%+ faster completion. No new science, just the integration the industry never shipped — running on a Raspberry Pi."*
