use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Html,
    routing::{get, post},
    Json, Router,
};
use crate::node::actor::RobotTelemetry;
use crate::protocol::{RobotId, TaskStatusMsg, Tick};
use crate::sim::SimEnvironment;
use crate::world::{Cell, GridMap, Pos};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;

#[derive(Clone)]
pub struct AppState {
    pub grid: Arc<GridMap>,
    pub environment: Arc<SimEnvironment>,
    pub telemetry_tx: broadcast::Sender<DashboardFrame>,
    pub control_queue: Arc<Mutex<Vec<ControlCommand>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ControlCommand {
    InjectObstacle(Pos),
    RemoveObstacle(Pos),
    ClearObstacles,
    KillRobot(RobotId),
    ReviveRobot(RobotId),
    SpawnTask,
    CustomTask { pickup: Pos, dropoff: Pos },
    ManualDispatch { robot_id: RobotId, target: Pos },
    LoadScenario(usize),
    SetSpeed(u64),
    ToggleContinuous(bool),
    ResetSim,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardFrame {
    pub tick: Tick,
    pub robots: Vec<RobotTelemetry>,
    pub obstacles: Vec<Pos>,
    pub tasks: Vec<TaskStatusMsg>,
    pub completed_count: usize,
    pub collisions: usize,
}

#[derive(Deserialize)]
pub struct ObstacleReq {
    pub x: usize,
    pub y: usize,
}

#[derive(Deserialize)]
pub struct RobotActionReq {
    pub robot_id: RobotId,
}

#[derive(Deserialize)]
pub struct CustomTaskReq {
    pub pickup_x: usize,
    pub pickup_y: usize,
    pub dropoff_x: usize,
    pub dropoff_y: usize,
}

#[derive(Deserialize)]
pub struct ManualDispatchReq {
    pub robot_id: RobotId,
    pub target_x: usize,
    pub target_y: usize,
}

#[derive(Deserialize)]
pub struct ScenarioReq {
    pub scenario_id: usize,
}

#[derive(Deserialize)]
pub struct SpeedReq {
    pub delay_ms: u64,
}

#[derive(Deserialize)]
pub struct ContinuousReq {
    pub enabled: bool,
}

const DASHBOARD_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>SIH26123 - Distributed AMR Mesh Command Console</title>
    <style>
        * { box-sizing: border-box; margin: 0; padding: 0; font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, monospace; }
        body { background: #080e1e; color: #f8fafc; display: flex; height: 100vh; overflow: hidden; }
        #sidebar { width: 400px; background: #0f172a; border-right: 1px solid #1e293b; padding: 18px; display: flex; flex-direction: column; gap: 12px; overflow-y: auto; }
        #main { flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: center; padding: 16px; position: relative; }
        canvas { background: #030712; border: 2px solid #3b82f6; border-radius: 8px; box-shadow: 0 12px 35px rgba(0,0,0,0.7); cursor: crosshair; }
        .card { background: #0b1329; border: 1px solid #1e293b; border-radius: 8px; padding: 12px; }
        .card h3 { color: #38bdf8; font-size: 12px; font-weight: 700; margin-bottom: 8px; text-transform: uppercase; letter-spacing: 0.5px; }
        .stat-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 8px; }
        .stat-box { background: #0f172a; padding: 8px 10px; border-radius: 6px; border: 1px solid #1e293b; }
        .stat-label { font-size: 10px; color: #94a3b8; text-transform: uppercase; }
        .stat-val { font-size: 20px; font-weight: bold; color: #10b981; margin-top: 2px; }
        .robot-item { display: flex; justify-content: space-between; align-items: center; padding: 6px 0; border-bottom: 1px solid #1e293b; font-size: 12px; }
        .badge { padding: 2px 7px; border-radius: 4px; font-size: 10px; font-weight: bold; }
        .badge-moving { background: #0284c7; color: white; }
        .badge-idle { background: #475569; color: white; }
        .badge-planning { background: #d97706; color: white; }
        .badge-yielding { background: #e11d48; color: white; }
        .badge-dead { background: #dc2626; color: white; }
        .btn-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 6px; }
        .btn { background: #2563eb; color: white; border: none; padding: 8px 10px; border-radius: 6px; cursor: pointer; font-size: 11px; font-weight: 600; text-align: center; transition: all 0.15s; }
        .btn:hover { opacity: 0.9; transform: translateY(-1px); }
        .btn:active { transform: translateY(0); }
        .btn-danger { background: #dc2626; }
        .btn-success { background: #059669; }
        .btn-secondary { background: #1e293b; color: #cbd5e1; border: 1px solid #334155; }
        .btn-active { background: #3b82f6 !important; color: white !important; border: 1px solid #60a5fa !important; }
        .tool-bar { display: flex; gap: 6px; margin-bottom: 10px; }
        .tool-btn { flex: 1; padding: 8px; font-size: 11px; font-weight: 700; border-radius: 6px; border: 1px solid #334155; background: #0f172a; color: #94a3b8; cursor: pointer; text-align: center; }
        .tool-btn.active { background: #1d4ed8; color: #ffffff; border-color: #60a5fa; box-shadow: 0 0 10px rgba(59,130,246,0.5); }
        .slider-container { display: flex; align-items: center; gap: 8px; font-size: 11px; color: #94a3b8; }
        .slider { flex: 1; accent-color: #3b82f6; cursor: pointer; }
        .task-row { display: flex; justify-content: space-between; font-size: 11px; padding: 4px 0; border-bottom: 1px solid #131f3d; color: #cbd5e1; }
    </style>
</head>
<body>
    <div id="sidebar">
        <div>
            <h1 style="font-size: 17px; color: #38bdf8; font-weight: 800;">SIH26123 P2P MESH</h1>
            <p style="font-size: 11px; color: #94a3b8;">ISO 3691-4 Fail-Safe Decentralized AMR Control</p>
        </div>

        <div class="card">
            <h3>Interactive Canvas Tool</h3>
            <div class="tool-bar">
                <button class="tool-btn active" id="tool-obs" onclick="setTool('obs')">🧱 Obstacle</button>
                <button class="tool-btn" id="tool-task" onclick="setTool('task')">📦 Dispatch</button>
                <button class="tool-btn" id="tool-manual" onclick="setTool('manual')">🤖 Move AMR</button>
            </div>
            <div id="tool-hint" style="font-size: 11px; color: #60a5fa; background: rgba(59,130,246,0.1); padding: 6px; border-radius: 4px; border: 1px solid rgba(59,130,246,0.2);">
                Click any cell to toggle dynamic obstacles.
            </div>
        </div>

        <div class="card">
            <h3>Preset Demo Scenarios</h3>
            <div style="display: flex; flex-direction: column; gap: 5px;">
                <button class="btn btn-secondary" onclick="loadScenario(1)">🏁 1. Head-On Corridor Bottleneck</button>
                <button class="btn btn-secondary" onclick="loadScenario(2)">🏁 2. 4-Way Gridlock Cycle Breaker</button>
                <button class="btn btn-secondary" onclick="loadScenario(3)">🏁 3. Multi-Task Fleet Rush</button>
            </div>
        </div>

        <div class="card">
            <h3>Fleet Performance</h3>
            <div class="stat-grid">
                <div class="stat-box">
                    <div class="stat-label">Tick</div>
                    <div class="stat-val" id="tick-val">0</div>
                </div>
                <div class="stat-box">
                    <div class="stat-label">Tasks Done</div>
                    <div class="stat-val" id="tasks-done" style="color: #38bdf8;">0</div>
                </div>
                <div class="stat-box">
                    <div class="stat-label">Collisions</div>
                    <div class="stat-val" id="collisions-val" style="color: #34d399;">0</div>
                </div>
                <div class="stat-box">
                    <div class="stat-label">Safety Status</div>
                    <div class="stat-val" style="color: #34d399; font-size: 13px; margin-top: 4px;">Zero Collisions</div>
                </div>
            </div>
        </div>

        <div class="card">
            <h3>Fleet Recovery & Chaos</h3>
            <div style="display: grid; grid-template-columns: repeat(2, 1fr); gap: 6px;" id="robot-toggle-btns"></div>
            <div class="btn-grid" style="margin-top: 8px;">
                <button class="btn btn-secondary" onclick="clearObstacles()">🧹 Clear Walls</button>
                <button class="btn btn-secondary" onclick="resetFleet()">🔄 Reset Fleet</button>
            </div>
            <div class="slider-container" style="margin-top: 10px;">
                <span>Speed:</span>
                <input type="range" min="20" max="400" value="120" class="slider" id="speed-slider" oninput="changeSpeed(this.value)">
                <span id="speed-label">120ms</span>
            </div>
        </div>

        <div class="card" style="flex: 1;">
            <h3>Active AMRs (<span id="robot-count">0</span>)</h3>
            <div id="robot-list"></div>
        </div>

        <div class="card" style="max-height: 140px; overflow-y: auto;">
            <h3>Live Auction & Task Pool</h3>
            <div id="task-list"></div>
        </div>
    </div>

    <div id="main">
        <canvas id="gridCanvas" width="660" height="660"></canvas>
        <div style="display: flex; gap: 14px; font-size: 11px; color: #94a3b8; margin-top: 8px;">
            <div style="display:flex; align-items:center; gap:4px;"><div style="width:10px;height:10px;background:#334155;border-radius:2px;"></div> Shelf</div>
            <div style="display:flex; align-items:center; gap:4px;"><div style="width:10px;height:10px;background:#ef4444;border-radius:2px;"></div> Injected Obstacle</div>
            <div style="display:flex; align-items:center; gap:4px;"><div style="width:10px;height:10px;background:#10b981;border-radius:2px;"></div> Pickup</div>
            <div style="display:flex; align-items:center; gap:4px;"><div style="width:10px;height:10px;background:#a855f7;border-radius:2px;"></div> Dropoff</div>
        </div>
    </div>

    <script>
        const canvas = document.getElementById('gridCanvas');
        const ctx = canvas.getContext('2d');
        let width = 15;
        let height = 15;
        let cellSize = canvas.width / width;

        let activeTool = 'obs'; // 'obs', 'task', 'manual'
        let taskPickup = null;
        let selectedRobotId = null;

        const colors = ['#38bdf8', '#4ade80', '#fbbf24', '#f472b6', '#a78bfa', '#fb7185'];

        function setTool(tool) {
            activeTool = tool;
            taskPickup = null;
            selectedRobotId = null;
            document.querySelectorAll('.tool-btn').forEach(b => b.classList.remove('active'));
            document.getElementById(`tool-${tool}`).classList.add('active');

            const hints = {
                'obs': '🧱 Click any cell to place or remove dynamic obstacles.',
                'task': '📦 Step 1: Click anywhere to set PICKUP point.',
                'manual': '🤖 Click an AMR on the map to select it, then click target.'
            };
            document.getElementById('tool-hint').innerText = hints[tool];
        }

        function isShelfWall(x, y) {
            return (y % 3 === 2) && (x >= 2 && x <= 4 || x >= 8 && x <= 10 || x >= 12 && x <= 13);
        }

        const ws = new WebSocket(`ws://${location.host}/ws`);
        ws.onmessage = (e) => {
            const frame = JSON.parse(e.data);
            document.getElementById('tick-val').innerText = frame.tick;
            document.getElementById('tasks-done').innerText = frame.completed_count;
            document.getElementById('collisions-val').innerText = frame.collisions;
            document.getElementById('robot-count').innerText = frame.robots.length;

            ctx.clearRect(0, 0, canvas.width, canvas.height);

            // Draw Zones
            for (let y = 0; y < height; y++) {
                for (let x = 0; x < width; x++) {
                    if (x < width / 3) {
                        ctx.fillStyle = 'rgba(16, 185, 129, 0.15)';
                        ctx.fillRect(x * cellSize, y * cellSize, cellSize, cellSize);
                    } else if (x >= (width * 2) / 3) {
                        ctx.fillStyle = 'rgba(168, 85, 247, 0.15)';
                        ctx.fillRect(x * cellSize, y * cellSize, cellSize, cellSize);
                    }
                }
            }

            // Draw Grid Lines
            ctx.strokeStyle = '#1e293b';
            ctx.lineWidth = 1;
            for (let i = 0; i <= width; i++) {
                ctx.beginPath();
                ctx.moveTo(i * cellSize, 0);
                ctx.lineTo(i * cellSize, canvas.height);
                ctx.stroke();
                ctx.beginPath();
                ctx.moveTo(0, i * cellSize);
                ctx.lineTo(canvas.width, i * cellSize);
                ctx.stroke();
            }

            // Draw Obstacles
            for (const obs of frame.obstacles) {
                const isShelf = isShelfWall(obs.x, obs.y);
                ctx.fillStyle = isShelf ? '#334155' : '#ef4444';
                ctx.fillRect(obs.x * cellSize + 2, obs.y * cellSize + 2, cellSize - 4, cellSize - 4);
                if (!isShelf) {
                    ctx.strokeStyle = '#fca5a5';
                    ctx.lineWidth = 2;
                    ctx.strokeRect(obs.x * cellSize + 2, obs.y * cellSize + 2, cellSize - 4, cellSize - 4);
                }
            }

            // Draw Staged Task Pickup selection if active
            if (taskPickup) {
                ctx.strokeStyle = '#10b981';
                ctx.lineWidth = 3;
                ctx.strokeRect(taskPickup.x * cellSize + 4, taskPickup.y * cellSize + 4, cellSize - 8, cellSize - 8);
                ctx.fillStyle = '#10b981';
                ctx.fillText('PICKUP', taskPickup.x * cellSize + cellSize/2, taskPickup.y * cellSize + cellSize/2);
            }

            // Draw Tasks
            if (frame.tasks) {
                const taskListEl = document.getElementById('task-list');
                taskListEl.innerHTML = '';
                for (const t of frame.tasks) {
                    if (t.status !== 'Completed') {
                        // Pickup
                        ctx.strokeStyle = '#10b981';
                        ctx.lineWidth = 2;
                        ctx.strokeRect(t.pickup.x * cellSize + 4, t.pickup.y * cellSize + 4, cellSize - 8, cellSize - 8);
                        // Dropoff
                        ctx.strokeStyle = '#a855f7';
                        ctx.lineWidth = 2;
                        ctx.strokeRect(t.dropoff.x * cellSize + 4, t.dropoff.y * cellSize + 4, cellSize - 8, cellSize - 8);

                        taskListEl.innerHTML += `
                            <div class="task-row">
                                <span>Task #${t.task_id}</span>
                                <span>(${t.pickup.x},${t.pickup.y}) → (${t.dropoff.x},${t.dropoff.y})</span>
                                <span style="color:${t.assigned_to ? '#38bdf8':'#fbbf24'}">${t.assigned_to ? 'R'+t.assigned_to : 'Open'}</span>
                            </div>
                        `;
                    }
                }
            }

            // Draw Robots & Space-Time Paths
            const list = document.getElementById('robot-list');
            const btnsContainer = document.getElementById('robot-toggle-btns');
            list.innerHTML = '';
            btnsContainer.innerHTML = '';

            for (const r of frame.robots) {
                const col = colors[(r.id - 1) % colors.length];

                // Selected ring
                if (selectedRobotId === r.id) {
                    ctx.strokeStyle = '#facc15';
                    ctx.lineWidth = 3;
                    ctx.beginPath();
                    ctx.arc(r.pos.x * cellSize + cellSize/2, r.pos.y * cellSize + cellSize/2, cellSize/2 + 2, 0, Math.PI*2);
                    ctx.stroke();
                }

                // Planned Path Line
                if (r.path && r.path.length > 0) {
                    ctx.strokeStyle = col;
                    ctx.lineWidth = 3;
                    ctx.beginPath();
                    ctx.moveTo(r.pos.x * cellSize + cellSize/2, r.pos.y * cellSize + cellSize/2);
                    for (const p of r.path) {
                        ctx.lineTo(p.x * cellSize + cellSize/2, p.y * cellSize + cellSize/2);
                    }
                    ctx.stroke();

                    const lastP = r.path[r.path.length - 1];
                    ctx.fillStyle = col;
                    ctx.beginPath();
                    ctx.arc(lastP.x * cellSize + cellSize/2, lastP.y * cellSize + cellSize/2, 4, 0, Math.PI * 2);
                    ctx.fill();
                }

                // Robot Body
                ctx.fillStyle = r.status === 'Dead' ? '#dc2626' : col;
                ctx.beginPath();
                ctx.arc(r.pos.x * cellSize + cellSize/2, r.pos.y * cellSize + cellSize/2, cellSize/2 - 4, 0, Math.PI * 2);
                ctx.fill();

                ctx.strokeStyle = '#ffffff';
                ctx.lineWidth = 2;
                ctx.stroke();

                ctx.fillStyle = '#000';
                ctx.font = 'bold 11px sans-serif';
                ctx.textAlign = 'center';
                ctx.textBaseline = 'middle';
                ctx.fillText(`R${r.id}`, r.pos.x * cellSize + cellSize/2, r.pos.y * cellSize + cellSize/2);

                let badgeClass = 'badge-idle';
                if (r.status === 'Moving') badgeClass = 'badge-moving';
                if (r.status === 'Planning') badgeClass = 'badge-planning';
                if (r.status === 'Yielding') badgeClass = 'badge-yielding';
                if (r.status === 'Dead') badgeClass = 'badge-dead';

                list.innerHTML += `
                    <div class="robot-item" onclick="selectRobotDirect(${r.id})" style="cursor:pointer;">
                        <div>
                            <span style="color:${col}; font-weight:bold;">AMR-${r.id}</span>
                            <span style="color:#64748b; font-size:11px; margin-left:2px;">(${r.pos.x},${r.pos.y})</span>
                        </div>
                        <div>
                            <span class="badge ${badgeClass}">${r.status}</span>
                            <span style="font-size:11px; color:#94a3b8; margin-left:4px;">${Math.round(r.battery * 100)}%</span>
                        </div>
                    </div>
                `;

                if (r.status === 'Dead') {
                    btnsContainer.innerHTML += `<button class="btn btn-success" onclick="reviveRobot(${r.id})">💚 Revive R${r.id}</button>`;
                } else {
                    btnsContainer.innerHTML += `<button class="btn btn-danger" onclick="killRobot(${r.id})">⚡ Kill R${r.id}</button>`;
                }
            }
        };

        canvas.addEventListener('click', (e) => {
            const rect = canvas.getBoundingClientRect();
            const x = Math.floor((e.clientX - rect.left) / cellSize);
            const y = Math.floor((e.clientY - rect.top) / cellSize);

            if (activeTool === 'obs') {
                fetch('/api/obstacle', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ x, y })
                });
            } else if (activeTool === 'task') {
                if (!taskPickup) {
                    taskPickup = { x, y };
                    document.getElementById('tool-hint').innerText = `📦 Step 2: Click anywhere to set DROPOFF point for Pickup at (${x},${y}).`;
                } else {
                    fetch('/api/custom-task', {
                        method: 'POST',
                        headers: { 'Content-Type': 'application/json' },
                        body: JSON.stringify({
                            pickup_x: taskPickup.x,
                            pickup_y: taskPickup.y,
                            dropoff_x: x,
                            dropoff_y: y
                        })
                    });
                    taskPickup = null;
                    document.getElementById('tool-hint').innerText = '✅ Custom Task Dispatched to Auction Pool! Click for new task.';
                }
            } else if (activeTool === 'manual') {
                if (!selectedRobotId) {
                    // Try to select robot at x,y
                    // Handled or prompt to click target
                    document.getElementById('tool-hint').innerText = '🤖 Select a robot from the sidebar or click target position.';
                } else {
                    fetch('/api/manual-dispatch', {
                        method: 'POST',
                        headers: { 'Content-Type': 'application/json' },
                        body: JSON.stringify({ robot_id: selectedRobotId, target_x: x, target_y: y })
                    });
                    document.getElementById('tool-hint').innerText = `✅ Dispatched AMR-${selectedRobotId} to (${x}, ${y})!`;
                    selectedRobotId = null;
                }
            }
        });

        function selectRobotDirect(id) {
            selectedRobotId = id;
            setTool('manual');
            document.getElementById('tool-hint').innerText = `🤖 AMR-${id} selected! Click any cell on the grid to send it there.`;
        }

        function loadScenario(id) {
            fetch('/api/scenario', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ scenario_id: id })
            });
        }

        function killRobot(id) {
            fetch('/api/kill', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ robot_id: id })
            });
        }

        function reviveRobot(id) {
            fetch('/api/revive', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ robot_id: id })
            });
        }

        function resetFleet() {
            fetch('/api/reset', { method: 'POST' });
        }

        function clearObstacles() {
            fetch('/api/clear-obstacles', { method: 'POST' });
        }

        function changeSpeed(val) {
            document.getElementById('speed-label').innerText = val + 'ms';
            fetch('/api/speed', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ delay_ms: parseInt(val) })
            });
        }
    </script>
</body>
</html>
"#;

async fn index_handler() -> Html<&'static str> {
    Html(DASHBOARD_HTML)
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> axum::response::Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: AppState) {
    let mut rx = state.telemetry_tx.subscribe();
    while let Ok(frame) = rx.recv().await {
        if let Ok(msg) = serde_json::to_string(&frame) {
            if socket.send(Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    }
}

async fn obstacle_handler(
    State(state): State<AppState>,
    Json(req): Json<ObstacleReq>,
) -> Json<&'static str> {
    let pos = Pos::new(req.x, req.y);
    let mut q = state.control_queue.lock().unwrap();
    let current_cell = state.environment.ground_truth.read().unwrap().get_cell(pos);
    if current_cell == Cell::Wall {
        q.push(ControlCommand::RemoveObstacle(pos));
    } else {
        q.push(ControlCommand::InjectObstacle(pos));
    }
    Json("Obstacle toggled")
}

async fn custom_task_handler(
    State(state): State<AppState>,
    Json(req): Json<CustomTaskReq>,
) -> Json<&'static str> {
    let mut q = state.control_queue.lock().unwrap();
    q.push(ControlCommand::CustomTask {
        pickup: Pos::new(req.pickup_x, req.pickup_y),
        dropoff: Pos::new(req.dropoff_x, req.dropoff_y),
    });
    Json("Custom task queued")
}

async fn manual_dispatch_handler(
    State(state): State<AppState>,
    Json(req): Json<ManualDispatchReq>,
) -> Json<&'static str> {
    let mut q = state.control_queue.lock().unwrap();
    q.push(ControlCommand::ManualDispatch {
        robot_id: req.robot_id,
        target: Pos::new(req.target_x, req.target_y),
    });
    Json("Manual dispatch queued")
}

async fn scenario_handler(
    State(state): State<AppState>,
    Json(req): Json<ScenarioReq>,
) -> Json<&'static str> {
    let mut q = state.control_queue.lock().unwrap();
    q.push(ControlCommand::LoadScenario(req.scenario_id));
    Json("Scenario queued")
}

async fn clear_obstacles_handler(State(state): State<AppState>) -> Json<&'static str> {
    let mut q = state.control_queue.lock().unwrap();
    q.push(ControlCommand::ClearObstacles);
    Json("Obstacles cleared")
}

async fn kill_handler(
    State(state): State<AppState>,
    Json(req): Json<RobotActionReq>,
) -> Json<&'static str> {
    let mut q = state.control_queue.lock().unwrap();
    q.push(ControlCommand::KillRobot(req.robot_id));
    Json("Kill command queued")
}

async fn revive_handler(
    State(state): State<AppState>,
    Json(req): Json<RobotActionReq>,
) -> Json<&'static str> {
    let mut q = state.control_queue.lock().unwrap();
    q.push(ControlCommand::ReviveRobot(req.robot_id));
    Json("Revive command queued")
}

async fn reset_handler(State(state): State<AppState>) -> Json<&'static str> {
    let mut q = state.control_queue.lock().unwrap();
    q.push(ControlCommand::ResetSim);
    Json("Reset command queued")
}

async fn speed_handler(
    State(state): State<AppState>,
    Json(req): Json<SpeedReq>,
) -> Json<&'static str> {
    let mut q = state.control_queue.lock().unwrap();
    q.push(ControlCommand::SetSpeed(req.delay_ms));
    Json("Speed set")
}

pub async fn start_dashboard_server(
    port: u16,
    grid: Arc<GridMap>,
    environment: Arc<SimEnvironment>,
    telemetry_tx: broadcast::Sender<DashboardFrame>,
    control_queue: Arc<Mutex<Vec<ControlCommand>>>,
) {
    let state = AppState {
        grid,
        environment,
        telemetry_tx,
        control_queue,
    };

    let app = Router::new()
        .route("/", get(index_handler))
        .route("/ws", get(ws_handler))
        .route("/api/obstacle", post(obstacle_handler))
        .route("/api/custom-task", post(custom_task_handler))
        .route("/api/manual-dispatch", post(manual_dispatch_handler))
        .route("/api/scenario", post(scenario_handler))
        .route("/api/clear-obstacles", post(clear_obstacles_handler))
        .route("/api/kill", post(kill_handler))
        .route("/api/revive", post(revive_handler))
        .route("/api/reset", post(reset_handler))
        .route("/api/speed", post(speed_handler))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    println!("Web Dashboard listening on http://localhost:{}", port);
    axum::serve(listener, app).await.unwrap();
}
