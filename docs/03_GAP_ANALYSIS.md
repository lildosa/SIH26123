# SIH26123 — Gap Analysis: Why Decentralized Fleet Coordination Is Still Unsolved

---

## The Gap (precisely stated)

> **No production system coordinates AMR fleets peer-to-peer. Every deployed stack (OTTO, MiR, openTCS, Open-RMF, anything VDA5050) has a central arbiter holding global truth. Decentralized coordination exists only as ~20 years of academic papers validated in sims with idealized Wi-Fi. The gap is: nobody has closed the sim-to-factory-floor distance for a fully decentralized fleet.**

---

## Why It's Still Unsolved — 4 Locks on the Door

### Lock 1 — Global guarantees need global information (technical)

"Zero collisions" and "no deadlock" are **global properties** of the fleet's joint state.

- A central server sees all; a robot sees only neighbors over lossy radio
- Distributed deadlock detection is provably nasty (Chandy–Misra–Haas assumes reliable messaging; warehouse Wi-Fi doesn't)
- Consequences: false deadlocks, stale intents, reservation races
- Papers sidestep this by assuming perfect comms; factories can't

### Lock 2 — Certification requires determinism (regulatory)

ISO 3691-4 — the safety standard behind CE marking for driverless industrial trucks (harmonized under EU Machinery Directive) — demands **verifiable, auditable safety behavior**.

- You certify *one* central controller by testing it
- An emergent decentralized swarm has a combinatorial state space that cannot be exhaustively verified
- Certifiers won't sign → insurers won't cover → factories won't deploy

References:
- ISO 3691-4:2023: https://www.iso.org/standard/70660.html
- Explainer (CE marking link): https://www.fabrico.io/blog/iso-3691-4-driverless-industrial-trucks/
- Certification practice (SGS): https://www.sgs.com.tw/en/news-media-resources-content/page/1?id=2135

### Lock 3 — Vendor economics reward the center (commercial)

- Robot vendors sell robot + proprietary FMS as a bundle; the FMS is where margin and lock-in live
- A peer protocol commoditizes their most defensible layer
- VDA5050 "interoperability" standardizes only the spokes-to-hub interface — it *reifies* the center rather than removing it

Reference: https://www.vda.de/en/topics/automotive-industry/vda-5050

### Lock 4 — Liability needs a defendant (legal)

- Central FMS: one accountable party, one audit log
- N autonomous peers: liability is diffuse
- Law lags technology → procurement defaults to the accountable-center model

---

## Root Cause (one sentence beneath all four locks)

> **The safety/liveness properties customers demand are *global* and *deterministic*, while decentralization fundamentally offers only *local* views over *non-deterministic* channels. Everything else — no certification path, no vendor incentive, no liability framework — cascades from that mismatch.**

This is a **socio-technical deadlock**, ironically:

- The technical fix (robust distributed protocols) was never productized because certification/economics blocked deployment
- Certification never evolved because nothing was deployed
- Each side waits for the other

---

## Why It's Solvable NOW (timing argument)

1. Edge compute got cheap enough (Jetson/Pi ≈ free relative to robot cost) — PS explicitly names them
2. ROS 2 made DDS peer discovery mainstream; mesh radios matured
3. Warehouse density exploded → central-server latency/bandwidth ceilings became *real money*, not theory
4. Academic pieces (CBS, auctions, reservation tables) are mature and off-the-shelf

---

## Implications for Our Solution

| Lock | Our answer |
|---|---|
| 1. Global vs local | Local wait-for graphs + priority/aging = liveness without global view (PLAN.md §5); deterministic tiebreaks |
| 2. Certification | Certification-*ready by design*: deterministic priority rules, full per-robot audit logs, every decision reproducible from robot-local logs |
| 3. Economics | Runs on ₹5k-class hardware; vendor-neutral protocol = anti-lock-in selling point for BEL |
| 4. Liability | Passive dashboard + immutable local logs give an audit trail without a coordinator |
