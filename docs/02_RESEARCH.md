# SIH26123 — Research: Existing Solutions & References

> What already exists for decentralized multi-robot fleet coordination, organized in 4 layers.
> **Headline finding:** every *deployed* system coordinates via a central arbiter; decentralization exists only in research.

---

## 1. Commercial Fleet Managers — all centralized (the "old way")

| Solution | What it is | Link |
|---|---|---|
| **OTTO Motors (Rockwell Automation)** | AMR fleet manager, central traffic control | https://ottomotors.com |
| **MiR Fleet / MiR Navigate** | Mobile Industrial Robots' centralized FMS | https://mobile-industrial-robots.com |
| **Locus Robotics** | Vendor-locked server-based fleet orchestration | https://locusrobotics.com |
| **Geek+** | Goods-to-person AMR + central FMS | https://www.geekplus.com |

All confirm the PS's premise: industry standard = central server + proprietary stack → exactly the single-point-of-failure we remove.

## 2. Open Source & Standards — still centralized at heart

| Solution | Relevance | Link |
|---|---|---|
| **openTCS** (Fraunhofer IML) | Reference open-source AGV fleet framework — routing/dispatching/scheduling all inside a **central kernel**. Study its scheduler logic, then invert it. Built-in VDA5050 support | https://github.com/openTCS/opentcs · https://www.opentcs.org |
| **Open-RMF** (Open Robotics) | Closest OSS to this PS: space-time itinerary schedule + conflict **negotiations** between fleets. But explicitly *"cooperative and decentralized in spirit, centralized in arbitration"* | https://github.com/open-rmf · https://www.open-rmf.org |
| **VDA 5050** | Industry protocol standard — MQTT between a *central master control* and AGVs/AMRs. Explicitly states traffic/collision logic is the **fleet manager's job, not peer-to-peer** — institutionalizes the centralized model | https://www.vda.de/en/topics/automotive-industry/vda-5050 · https://github.com/VDA5050/VDA5050 |
| **ROS 2 Nav2** | Per-robot navigation stack (single robot solved); no fleet coordination built in | https://docs.nav2.org |

## 3. Academic Building Blocks (what PLAN.md borrows)

| Technique | Paper / Source | Link |
|---|---|---|
| Reservation tables + cooperative pathfinding | Silver, *"Cooperative Pathfinding"*, AIIDE 2005 | https://dl.acm.org/doi/10.5555/2900728.2900809 |
| Optimal MAPF (stretch goal) | Sharon, Stern, Felner, Sturtevant, *"Conflict-Based Search"*, AAAI 2012 / AIJ 2015 (~2000+ citations) | https://ojs.aaai.org/index.php/AAAI/article/view/8140 |
| Auction task allocation (foundation) | Smith, *"Contract Net Protocol"*, IEEE Trans. Computers 1980 | https://en.wikipedia.org/wiki/Contract_Net_Protocol |
| Auctions validated on real robots | Gerkey & Matarić, *"Sold!: Auction Methods for Multi-Robot Coordination"* (MURDOCH), IEEE T-RA 2002 | https://robotics.stanford.edu/~gerkey/research/murdoch.html · https://ieeexplore.ieee.org/document/9646189 (combinatorial variant) |
| Market-based coordination survey | Dias et al., *"TraderBots"*, CMU-RI-TR 2004/2005 | https://publications.ri.cmu.edu/storage/publications/pub_files/pub4/dias_m_bernardine_2005_5/dias_m_bernardine_2005_5.pdf |
| MRTA taxonomy / recent survey | *"A Systematic Literature Review on Multi-Robot Task Allocation"*, ACM Computing Surveys 2024 | https://dl.acm.org/doi/10.1145/3700591 |

## 4. Decentralized Research — the actual frontier (closest competitors)

| Work | What exists | Link |
|---|---|---|
| Draganjac et al., IEEE TASE 2016 → RCIM 2020 | Fully decentralized multi-AGV traffic control, private-zone conflict avoidance, sim + real experiments; explicitly targets eliminating the single point of failure | https://www.sciencedirect.com/science/article/abs/pii/S0736584518303843 |
| **MCCA**, IEEE RA-L 2024 | Masked Cooperative Collision Avoidance — fully decentralized collision **and deadlock** avoidance in dense narrow environments | https://ieeexplore.ieee.org/document/10414179 |
| Decentralized Deadlock Prevention, 2022 | Self-organizing industrial mobile robot fleets; zone-based task assignment + path coordination | https://www.researchgate.net/publication/362817330 |
| MARL deadlock handling, 2025 | Centralized-training/decentralized-execution RL vs rule-based deadlock handling | https://arxiv.org/abs/2511.07071 |
| ROS2/Nav2 lane-based multi-AMR traffic, 2024 | Keeps global planner onboard each AMR (avoids central scalability issue); traffic lanes via layered costmaps | https://www.sciencedirect.com/science/article/pii/S1877050924000061 |
| Deadlock/collision ABM study, 2022 | Agent-based modelling of AMR travel with prevention algorithms | https://www.tandfonline.com/doi/full/10.1080/13675567.2022.2138290 |

## 5. Gap = Our Pitch

> **Every deployed system (commercial, OSS, standards) coordinates via a central arbiter. Fully-decentralized edge coordination exists only as scattered papers with simplifying assumptions — no integrated, demoable, metric-proven framework. That integrated product IS SIH26123.**

Judges-safe framing: we are not inventing new algorithms (CBS/auctions/reservations are 20+ years old); we do novel **systems integration** of proven pieces under hard decentralization constraints with live metrics — precisely what BEL asked for.

*(See `03_GAP_ANALYSIS.md` for why the gap persists and its root cause.)*
