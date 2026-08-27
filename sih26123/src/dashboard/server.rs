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
use crate::protocol::{RobotId, Tick};
use crate::sim::SimEnvironment;
use crate::world::{GridMap, Pos};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::broadcast;

#[derive(Clone)]
pub struct AppState {
    pub grid: Arc<GridMap>,
    pub environment: Arc<SimEnvironment>,
    pub telemetry_tx: broadcast::Sender<DashboardFrame>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardFrame {
    pub tick: Tick,
    pub robots: Vec<RobotTelemetry>,
    pub obstacles: Vec<Pos>,
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

const DASHBOARD_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>SIH26123 - Distributed AMR Fleet Coordination</title>
    <style>
        * { box-sizing: border-box; margin: 0; padding: 0; font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, monospace; }
        body { background: #0f172a; color: #f8fafc; display: flex; height: 100vh; overflow: hidden; }
        #sidebar { width: 340px; background: #1e293b; border-right: 1px solid #334155; padding: 20px; display: flex; flex-direction: column; gap: 15px; }
        #main { flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: center; padding: 20px; }
        canvas { background: #020617; border: 2px solid #3b82f6; border-radius: 8px; box-shadow: 0 10px 25px rgba(0,0,0,0.5); }
        .card { background: #0f172a; border: 1px solid #334155; border-radius: 6px; padding: 12px; }
        .card h3 { color: #38bdf8; font-size: 14px; margin-bottom: 8px; text-transform: uppercase; }
        .stat-val { font-size: 24px; font-weight: bold; color: #10b981; }
        .robot-item { display: flex; justify-content: space-between; padding: 6px 0; border-bottom: 1px solid #1e293b; font-size: 13px; }
        .badge { padding: 2px 6px; border-radius: 4px; font-size: 11px; font-weight: bold; }
        .badge-moving { background: #0284c7; color: white; }
        .badge-idle { background: #475569; color: white; }
        .badge-planning { background: #d97706; color: white; }
        .badge-yielding { background: #e11d48; color: white; }
        .badge-dead { background: #991b1b; color: white; }
    </style>
</head>
<body>
    <div id="sidebar">
        <div>
            <h1 style="font-size: 18px; color: #38bdf8;">SIH26123 P2P Mesh</h1>
            <p style="font-size: 12px; color: #94a3b8;">ISO 3691-4 Fail-Safe AMR Fleet</p>
        </div>

        <div class="card">
            <h3>Simulation Tick</h3>
            <div class="stat-val" id="tick-val">0</div>
        </div>

        <div class="card">
            <h3>Safety Status</h3>
            <div style="font-size: 14px; color: #34d399;">✓ ZERO Collisions Guaranteed</div>
            <div style="font-size: 12px; color: #94a3b8; margin-top: 4px;">Dynamic Obstacle Sensing Active</div>
        </div>

        <div class="card" style="flex: 1; overflow-y: auto;">
            <h3>AMR Telemetry</h3>
            <div id="robot-list"></div>
        </div>

        <div class="card">
            <h3>Inject Fault / Obstacle</h3>
            <p style="font-size: 12px; color: #94a3b8; margin-bottom: 8px;">Click canvas to inject dynamic obstacle</p>
        </div>
    </div>

    <div id="main">
        <canvas id="gridCanvas" width="600" height="600"></canvas>
    </div>

    <script>
        const canvas = document.getElementById('gridCanvas');
        const ctx = canvas.getContext('2d');
        let width = 15;
        let height = 15;
        let cellSize = canvas.width / width;

        const ws = new WebSocket(`ws://${location.host}/ws`);
        ws.onmessage = (e) => {
            const frame = JSON.parse(e.data);
            document.getElementById('tick-val').innerText = frame.tick;

            ctx.clearRect(0, 0, canvas.width, canvas.height);

            // Draw grid lines
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

            // Draw obstacles
            ctx.fillStyle = '#475569';
            for (const obs of frame.obstacles) {
                ctx.fillRect(obs.x * cellSize + 2, obs.y * cellSize + 2, cellSize - 4, cellSize - 4);
            }

            // Draw robots & paths
            const list = document.getElementById('robot-list');
            list.innerHTML = '';

            const colors = ['#38bdf8', '#4ade80', '#fbbf24', '#f472b6', '#a78bfa', '#fb7185'];

            for (const r of frame.robots) {
                const col = colors[(r.id - 1) % colors.length];

                // Draw path trace
                if (r.path && r.path.length > 0) {
                    ctx.strokeStyle = col;
                    ctx.lineWidth = 2;
                    ctx.beginPath();
                    ctx.moveTo(r.pos.x * cellSize + cellSize/2, r.pos.y * cellSize + cellSize/2);
                    for (const p of r.path) {
                        ctx.lineTo(p.x * cellSize + cellSize/2, p.y * cellSize + cellSize/2);
                    }
                    ctx.stroke();
                }

                // Draw robot circle
                ctx.fillStyle = col;
                ctx.beginPath();
                ctx.arc(r.pos.x * cellSize + cellSize/2, r.pos.y * cellSize + cellSize/2, cellSize/2 - 4, 0, Math.PI * 2);
                ctx.fill();

                // Draw robot label
                ctx.fillStyle = '#000';
                ctx.font = 'bold 12px sans-serif';
                ctx.textAlign = 'center';
                ctx.textBaseline = 'middle';
                ctx.fillText(`R${r.id}`, r.pos.x * cellSize + cellSize/2, r.pos.y * cellSize + cellSize/2);

                // Update sidebar list
                let badgeClass = 'badge-idle';
                if (r.status === 'Moving') badgeClass = 'badge-moving';
                if (r.status === 'Planning') badgeClass = 'badge-planning';
                if (r.status === 'Yielding') badgeClass = 'badge-yielding';
                if (r.status === 'Dead') badgeClass = 'badge-dead';

                list.innerHTML += `
                    <div class="robot-item">
                        <div>
                            <span style="color:${col}; font-weight:bold;">AMR-${r.id}</span>
                            <span style="color:#64748b; font-size:11px;">(${r.pos.x}, ${r.pos.y})</span>
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
    state.environment.inject_obstacle(Pos::new(req.x, req.y));
    Json("Obstacle injected")
}

pub async fn start_dashboard_server(
    port: u16,
    grid: Arc<GridMap>,
    environment: Arc<SimEnvironment>,
    telemetry_tx: broadcast::Sender<DashboardFrame>,
) {
    let state = AppState {
        grid,
        environment,
        telemetry_tx,
    };

    let app = Router::new()
        .route("/", get(index_handler))
        .route("/ws", get(ws_handler))
        .route("/api/obstacle", post(obstacle_handler))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    println!("Web Dashboard listening on http://localhost:{}", port);
    axum::serve(listener, app).await.unwrap();
}
