Here is what each one actually does in plain English: 

 1 → Space-Time A (3D GPS with a clock).* Found in space_time_a_star.rs. Normal GPS only cares where you go in 2D (x, y), but this algorithm adds a time axis t. It plans routes like a 
 calendar ("I will be at coordinate (3, 4) at tick 10"), letting robots wait in place for a second so they never crash or swap spots head-on. 

 2 → DFS Cycle Detection (The standoff breaker). Found in wait_for_graph.rs. When Robot A is waiting for Robot B, and B is waiting for C, and C is waiting for A, they are stuck in a 
 Mexican standoff. This algorithm maps out who is waiting on whom and runs a quick graph search to detect that closed loop (R₁ → R₂ → R₃ → R₁) immediately. 

 3 → Deterministic Priority Arbitration (The right-of-way rulebook). Found in conflict.rs. When two robots want the exact same spot, we do not waste time arguing over the radio. They 
 check a simple rule: lower priority number wins, and ties break by lower Robot ID. Both robots do the exact same math, so the loser immediately yields and reroutes with zero back-and- 
 forth chatter. 

 4 → Contract Net Protocol (eBay for warehouse tasks). Found in contract_net.rs. When an order comes in, idle robots bid on it. Their bid price is calculated from how far they must drive, 
 local aisle traffic, battery level, and deadlines. The lowest-cost bidder wins the job automatically. 

 5 → Sequence Tracking & Heartbeat Detector (The spam filter and dead-man switch). Found in actor.rs:175-186. Every UDP message gets a counter so robots throw away duplicated or late- 
 arriving packets. If a peer goes silent for 5 ticks in a row, the fleet assumes it broke down, treats its body like a permanent wall, and re-auctions its unfinished tasks. 

 6 → Conflict-Based Search (The high-end academic baseline). Found in cbs.rs. This is the gold-standard algorithm from research papers that plans everything on a big central computer by 
 building a giant conflict tree. It is mathematically optimal, but it needs a heavy central server, which is why we use it as our benchmark target. 

 7 → Centralized FIFO Dispatcher (The standard factory baseline). Found in centralized.rs. This is the simple first-come, first-served queue that commercial warehouses typically run on a 
 central server. We built it strictly to prove our decentralized auctions beat traditional server queues. 


 
