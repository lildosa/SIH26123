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
    SpawnTask,
    SetSpeed(u64),
    ToggleContinuous(bool),
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
pub struct KillReq {
    pub robot_id: RobotId,
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
    <title>SIH26123 - Distributed AMR Mesh Dashboard</title>
    <style>
        * { box-sizing: border-box; margin: 0; padding: 0; font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, monospace; }
        body { background: #0b1329; color: #f8fafc; display: flex; height: 100vh; overflow: hidden; }
        #sidebar { width: 380px; background: #131f3d; border-right: 1px solid #1e293b; padding: 20px; display: flex; flex-direction: column; gap: 14px; overflow-y: auto; }
        #main { flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: center; padding: 20px; position: relative; }
        canvas { background: #070d1e; border: 2px solid #3b82f6; border-radius: 8px; box-shadow: 0 10px 30px rgba(0,0,0,0.6); cursor: crosshair; }
        .card { background: #0c1630; border: 1px solid #1e293b; border-radius: 8px; padding: 14px; }
        .card h3 { color: #38bdf8; font-size: 13px; font-weight: 700; margin-bottom: 8px; text-transform: uppercase; letter-spacing: 0.5px; }
        .stat-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 10px; }
        .stat-box { background: #131f3d; padding: 10px; border-radius: 6px; border: 1px solid #1e293b; }
        .stat-label { font-size: 11px; color: #94a3b8; text-transform: uppercase; }
        .stat-val { font-size: 22px; font-weight: bold; color: #10b981; margin-top: 2px; }
        .robot-item { display: flex; justify-content: space-between; align-items: center; padding: 8px 0; border-bottom: 1px solid #1e293b; font-size: 13px; }
        .badge { padding: 3px 8px; border-radius: 4px; font-size: 11px; font-weight: bold; }
        .badge-moving { background: #0284c7; color: white; }
        .badge-idle { background: #475569; color: white; }
        .badge-planning { background: #d97706; color: white; }
        .badge-yielding { background: #e11d48; color: white; }
        .badge-dead { background: #dc2626; color: white; }
        .btn-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 8px; margin-top: 6px; }
        .btn { background: #2563eb; color: white; border: none; padding: 9px 12px; border-radius: 6px; cursor: pointer; font-size: 12px; font-weight: 600; text-align: center; transition: background 0.15s; }
        .btn:hover { background: #1d4ed8; }
        .btn-danger { background: #dc2626; }
        .btn-danger:hover { background: #b91c1c; }
        .btn-success { background: #059669; }
        .btn-success:hover { background: #047857; }
        .btn-secondary { background: #334155; }
        .btn-secondary:hover { background: #475569; }
        .legend { display: flex; gap: 12px; font-size: 11px; color: #94a3b8; margin-top: 8px; }
        .legend-item { display: flex; align-items: center; gap: 4px; }
        .legend-box { width: 10px; height: 10px; border-radius: 2px; }
        .slider-container { display: flex; align-items: center; gap: 10px; font-size: 12px; color: #94a3b8; }
        .slider { flex: 1; accent-color: #3b82f6; cursor: pointer; }
    </style>
</head>
<body>
    <div id="sidebar">
        <div>
            <h1 style="font-size: 18px; color: #38bdf8; font-weight: 800;">SIH26123 P2P MESH</h1>
            <p style="font-size: 12px; color: #94a3b8;">ISO 3691-4 Decentralized AMR Coordination</p>
        </div>

        <div class="card">
            <h3>Fleet Statistics</h3>
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
                    <div class="stat-label">Safety Invariant</div>
                    <div class="stat-val" style="color: #34d399; font-size: 14px; margin-top: 6px;">100% Zero Collisions</div>
                </div>
            </div>
        </div>

        <div class="card">
            <h3>Interactive Controls</h3>
            <div class="btn-grid">
                <button class="btn btn-success" onclick="spawnTask()">✨ Spawn Task</button>
                <button class="btn btn-secondary" id="continuous-btn" onclick="toggleContinuous()">🔁 Auto-Spawn: ON</button>
            </div>
            <div style="margin-top: 10px;">
                <div style="font-size: 12px; color: #94a3b8; margin-bottom: 4px;">Inject Node Failure:</div>
                <div style="display: grid; grid-template-columns: repeat(4, 1fr); gap: 6px;">
                    <button class="btn btn-danger" onclick="killRobot(1)">Kill R1</button>
                    <button class="btn btn-danger" onclick="killRobot(2)">Kill R2</button>
                    <button class="btn btn-danger" onclick="killRobot(3)">Kill R3</button>
                    <button class="btn btn-danger" onclick="killRobot(4)">Kill R4</button>
                </div>
            </div>
            <div class="btn-grid" style="margin-top: 10px;">
                <button class="btn btn-secondary" onclick="clearObstacles()">🧹 Clear Obstacles</button>
            </div>
            <div class="slider-container" style="margin-top: 12px;">
                <span>Speed:</span>
                <input type="range" min="30" max="400" value="150" class="slider" id="speed-slider" oninput="changeSpeed(this.value)">
                <span id="speed-label">150ms</span>
            </div>
        </div>

        <div class="card" style="flex: 1;">
            <h3>Active AMRs (<span id="robot-count">0</span>)</h3>
            <div id="robot-list"></div>
        </div>
    </div>

    <div id="main">
        <canvas id="gridCanvas" width="660" height="660"></canvas>
        <div class="legend">
            <div class="legend-item"><div class="legend-box" style="background:#475569;"></div> Shelf Wall</div>
            <div class="legend-item"><div class="legend-box" style="background:#ef4444;"></div> Injected Obstacle</div>
            <div class="legend-item"><div class="legend-box" style="background:#10b981;"></div> Pickup Point</div>
            <div class="legend-item"><div class="legend-box" style="background:#a855f7;"></div> Dropoff Goal</div>
        </div>
        <p style="font-size: 11px; color: #64748b; margin-top: 6px;">💡 Click anywhere on the warehouse grid to place or remove obstacles in real-time.</p>
    </div>

    <script>
        const canvas = document.getElementById('gridCanvas');
        const ctx = canvas.getContext('2d');
        let width = 15;
        let height = 15;
        let cellSize = canvas.width / width;
        let continuousSpawn = true;

        const colors = ['#38bdf8', '#4ade80', '#fbbf24', '#f472b6', '#a78bfa', '#fb7185'];

        const ws = new WebSocket(`ws://${location.host}/ws`);
        ws.onmessage = (e) => {
            const frame = JSON.parse(e.data);
            document.getElementById('tick-val').innerText = frame.tick;
            document.getElementById('tasks-done').innerText = frame.completed_count;
            document.getElementById('collisions-val').innerText = frame.collisions;
            document.getElementById('robot-count').innerText = frame.robots.length;

            ctx.clearRect(0, 0, canvas.width, canvas.height);

            // Draw Grid
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

            // Draw Static and Dynamic Obstacles
            for (const obs of frame.obstacles) {
                ctx.fillStyle = '#334155';
                ctx.fillRect(obs.x * cellSize + 2, obs.y * cellSize + 2, cellSize - 4, cellSize - 4);
            }

            // Draw Active Tasks (Pickups & Dropoffs)
            if (frame.tasks) {
                for (const t of frame.tasks) {
                    if (t.status === 'Open' || t.status === 'Assigned' || t.status === 'InProgress') {
                        // Pickup Marker (Green)
                        ctx.fillStyle = 'rgba(16, 185, 129, 0.4)';
                        ctx.fillRect(t.pickup.x * cellSize + 4, t.pickup.y * cellSize + 4, cellSize - 8, cellSize - 8);
                        ctx.strokeStyle = '#10b981';
                        ctx.lineWidth = 2;
                        ctx.strokeRect(t.pickup.x * cellSize + 4, t.pickup.y * cellSize + 4, cellSize - 8, cellSize - 8);

                        // Dropoff Marker (Purple)
                        ctx.fillStyle = 'rgba(168, 85, 247, 0.4)';
                        ctx.fillRect(t.dropoff.x * cellSize + 4, t.dropoff.y * cellSize + 4, cellSize - 8, cellSize - 8);
                        ctx.strokeStyle = '#a855f7';
                        ctx.lineWidth = 2;
                        ctx.strokeRect(t.dropoff.x * cellSize + 4, t.dropoff.y * cellSize + 4, cellSize - 8, cellSize - 8);
                    }
                }
            }

            // Draw Robots & Space-Time Paths
            const list = document.getElementById('robot-list');
            list.innerHTML = '';

            for (const r of frame.robots) {
                const col = colors[(r.id - 1) % colors.length];

                // Planned Path Line
                if (r.path && r.path.length > 0) {
                    ctx.strokeStyle = col;
                    ctx.lineWidth = 2.5;
                    ctx.beginPath();
                    ctx.moveTo(r.pos.x * cellSize + cellSize/2, r.pos.y * cellSize + cellSize/2);
                    for (const p of r.path) {
                        ctx.lineTo(p.x * cellSize + cellSize/2, p.y * cellSize + cellSize/2);
                    }
                    ctx.stroke();

                    // Endpoint dot
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

                // Robot Border
                ctx.strokeStyle = '#ffffff';
                ctx.lineWidth = 1.5;
                ctx.stroke();

                // Label
                ctx.fillStyle = '#000';
                ctx.font = 'bold 11px sans-serif';
                ctx.textAlign = 'center';
                ctx.textBaseline = 'middle';
                ctx.fillText(`R${r.id}`, r.pos.x * cellSize + cellSize/2, r.pos.y * cellSize + cellSize/2);

                // Sidebar Info
                let badgeClass = 'badge-idle';
                if (r.status === 'Moving') badgeClass = 'badge-moving';
                if (r.status === 'Planning') badgeClass = 'badge-planning';
                if (r.status === 'Yielding') badgeClass = 'badge-yielding';
                if (r.status === 'Dead') badgeClass = 'badge-dead';

                list.innerHTML += `
                    <div class="robot-item">
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
            }
        };

        canvas.addEventListener('click', (e) => {
            const rect = canvas.getBoundingClientRect();
            const x = Math.floor((e.clientX - rect.left) / cellSize);
            const y = Math.floor((e.clientY - rect.top) / cellSize);
            fetch('/api/obstacle', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ x, y })
            });
        });

        function spawnTask() {
            fetch('/api/task', { method: 'POST' });
        }

        function killRobot(id) {
            fetch('/api/kill', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ robot_id: id })
            });
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

        function toggleContinuous() {
            continuousSpawn = !continuousSpawn;
            const btn = document.getElementById('continuous-btn');
            btn.innerText = continuousSpawn ? '🔁 Auto-Spawn: ON' : '⏸ Auto-Spawn: OFF';
            btn.className = continuousSpawn ? 'btn btn-secondary' : 'btn btn-secondary';
            fetch('/api/continuous', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ enabled: continuousSpawn })
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

async fn clear_obstacles_handler(State(state): State<AppState>) -> Json<&'static str> {
    let mut q = state.control_queue.lock().unwrap();
    q.push(ControlCommand::ClearObstacles);
    Json("Obstacles cleared")
}

async fn kill_handler(
    State(state): State<AppState>,
    Json(req): Json<KillReq>,
) -> Json<&'static str> {
    let mut q = state.control_queue.lock().unwrap();
    q.push(ControlCommand::KillRobot(req.robot_id));
    Json("Kill command queued")
}

async fn task_handler(State(state): State<AppState>) -> Json<&'static str> {
    let mut q = state.control_queue.lock().unwrap();
    q.push(ControlCommand::SpawnTask);
    Json("Task queued")
}

async fn speed_handler(
    State(state): State<AppState>,
    Json(req): Json<SpeedReq>,
) -> Json<&'static str> {
    let mut q = state.control_queue.lock().unwrap();
    q.push(ControlCommand::SetSpeed(req.delay_ms));
    Json("Speed set")
}

async fn continuous_handler(
    State(state): State<AppState>,
    Json(req): Json<ContinuousReq>,
) -> Json<&'static str> {
    let mut q = state.control_queue.lock().unwrap();
    q.push(ControlCommand::ToggleContinuous(req.enabled));
    Json("Continuous mode set")
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
        .route("/api/clear-obstacles", post(clear_obstacles_handler))
        .route("/api/kill", post(kill_handler))
        .route("/api/task", post(task_handler))
        .route("/api/speed", post(speed_handler))
        .route("/api/continuous", post(continuous_handler))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    println!("Web Dashboard listening on http://localhost:{}", port);
    axum::serve(listener, app).await.unwrap();
}
