# SIH26123 — Problem Understanding

> **Title:** Edge-AI Based Distributed Fleet Coordination for Autonomous Mobile Robots (AMRs) in Smart Warehouses
> **Org:** Bharat Electronics Limited (BEL) · **Category:** Software · **Theme:** Robotics and Drones
> Source: SIH 2026 Problem Statements PDF

---

## 1. The Problem in One Line

Warehouse AMR fleets depend on a centralized cloud/server for coordination — this creates latency, Wi-Fi dead-zone vulnerability, and a single point of failure. We must move coordination onto the robots themselves.

## 2. WHO

| Actor | Role |
|---|---|
| **Warehouse ops teams** | Run AMR fleets (e-commerce fulfillment, defense depots, manufacturing intralogistics). Suffer downtime when the central server or network fails |
| **BEL / Indian industry** | Want indigenous, vendor-independent fleet software; today dependent on foreign stacks (OTTO/Rockwell, MiR, Geek+) locked to their central servers |
| **End beneficiaries** | Any multi-robot facility: warehouses, factories, hospitals, ports |
| **We (builders)** | Must make each robot smart *on its own edge hardware* (RPi/Jetson Nano) — no brain in the cloud |

## 3. WHAT — the three named failure modes of centralization

1. **Latency** — round-trips to a server make split-second collision avoidance impossible
2. **Wi-Fi dead zones** — steel racking kills radio; robot loses contact → freezes → fleet stalls
3. **Single point of failure** — server dies = warehouse dies

### What we must build (PS requirements)

1. **Decentralized Communication** — P2P inter-robot messaging sharing position and intent, no central server
2. **Dynamic Multi-Agent Conflict Resolution** — real-time deadlock & collision handling at narrow intersections / choke points
3. **Task Allocation & Re-routing** — auto reassign pickups / replan paths when an aisle is blocked or a robot fails

### Expected deliverables (per PS)

- Multi-robot simulation
- Decentralized network stack (peer-to-peer localization sharing)
- Multi-agent path planning algorithms running on edge hardware
- Lightweight Fleet Dashboard — real-time positions + battery status

### Hard success criteria (measured, not claimed)

- **Zero** inter-robot collisions
- **≥20% reduction** in total task completion time vs stop-and-wait baseline on overlapping paths

## 4. WHY — the deeper rationale

Fleets grow faster than centralized coordination scales: every robot multiplies server load and radio congestion. Decentralization flips the model:

> **Coordination cost scales with neighbors, not fleet size — and any single node failure degrades gracefully instead of halting everything.**

Modern robotics is shifting to decentralized edge computing so robots make split-second decisions on the fly.

## 5. Golden Rules for the Solution

- The dashboard is a **listener only**. Any hidden central coordinator = disqualification logic
- Every decision must be reproducible from robot-local logs
- Success criteria are numeric → measure live during the demo, never just claim them

## 6. Sub-problem Breakdown (maps to PLAN.md §1)

| # | Sub-problem | Core challenge |
|---|---|---|
| 1 | Decentralized communication | No central server allowed |
| 2 | Multi-agent path planning | Robots must share space-time safely |
| 3 | Conflict & deadlock resolution | Choke points without a referee |
| 4 | Task allocation & re-routing | Blocked aisles, failed robots |
| 5 | Fleet dashboard | Observe only, never coordinate |

## 7. Essence

*"Make N dumb carts behave like one intelligent fleet using only the network between them."*
