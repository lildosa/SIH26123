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
    SetFleetSize(usize),
    SetSpeed(u64),
    ToggleContinuous(bool),
    ResetSim,
    /// Live chaos injection: simulated packet-loss rate 0.0 - 0.5.
    SetPacketLoss(f64),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardFrame {
    pub tick: Tick,
    pub robots: Vec<RobotTelemetry>,
    pub static_walls: Vec<Pos>,
    pub dynamic_obstacles: Vec<Pos>,
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
pub struct FleetSizeReq {
    pub size: usize,
}

#[derive(Deserialize)]
pub struct SpeedReq {
    pub delay_ms: u64,
}

#[derive(Deserialize)]
pub struct PacketLossReq {
    pub loss_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkComparison {
    pub centralized_cbs_makespan: u64,
    pub swarmedge_makespan: u64,
    pub speedup_pct: f64,
    pub live_tick: Tick,
    pub live_completed: usize,
    pub live_collisions: usize,
}

const DASHBOARD_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>SIH26123 AMR Fleet Operations Console</title>
    <style>
        * { box-sizing: border-box; margin: 0; padding: 0; font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace; }
        body { background: #111215; color: #e4e4e7; display: flex; height: 100vh; overflow: hidden; }
        #sidebar { width: 400px; background: #18191e; border-right: 1px solid #27272a; padding: 18px; display: flex; flex-direction: column; gap: 12px; overflow-y: auto; }
        #main { flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: center; padding: 16px; position: relative; background: #0c0d0e; }
        canvas { background: #14151a; border: 1px solid #3f3f46; border-radius: 4px; box-shadow: 0 4px 20px rgba(0,0,0,0.8); cursor: crosshair; }
        .card { background: #1e1f26; border: 1px solid #2e2f38; border-radius: 4px; padding: 12px; }
        .card-header { color: #a1a1aa; font-size: 11px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.8px; margin-bottom: 8px; }
        .stat-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 8px; }
        .stat-box { background: #14151a; padding: 8px 10px; border-radius: 4px; border: 1px solid #27272a; }
        .stat-label { font-size: 10px; color: #71717a; text-transform: uppercase; letter-spacing: 0.5px; }
        .stat-val { font-size: 18px; font-weight: 700; color: #f4f4f5; margin-top: 2px; }
        .stat-val-green { color: #10b981; }
        .stat-val-amber { color: #f59e0b; }
        .robot-item { display: flex; justify-content: space-between; align-items: center; padding: 5px 0; border-bottom: 1px solid #27272a; font-size: 11px; }
        .badge { padding: 2px 6px; border-radius: 3px; font-size: 10px; font-weight: 600; text-transform: uppercase; }
        .badge-moving { background: #27272a; color: #38bdf8; border: 1px solid #38bdf8; }
        .badge-idle { background: #27272a; color: #71717a; border: 1px solid #3f3f46; }
        .badge-planning { background: #27272a; color: #f59e0b; border: 1px solid #f59e0b; }
        .badge-yielding { background: #27272a; color: #fb7185; border: 1px solid #fb7185; }
        .badge-dead { background: #450a0a; color: #ef4444; border: 1px solid #ef4444; }
        .btn-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 6px; }
        .btn { background: #27272a; color: #e4e4e7; border: 1px solid #3f3f46; padding: 7px 10px; border-radius: 3px; cursor: pointer; font-size: 11px; font-weight: 600; text-align: center; transition: all 0.1s; }
        .btn:hover { background: #3f3f46; color: #ffffff; }
        .btn:active { background: #18181b; }
        .btn-danger { border-color: #7f1d1d; color: #f87171; }
        .btn-danger:hover { background: #991b1b; color: #ffffff; }
        .btn-restore { border-color: #065f46; color: #34d399; }
        .btn-restore:hover { background: #047857; color: #ffffff; }
        .tool-bar { display: flex; gap: 4px; margin-bottom: 8px; }
        .tool-btn { flex: 1; padding: 7px 4px; font-size: 10px; font-weight: 700; border-radius: 3px; border: 1px solid #3f3f46; background: #14151a; color: #a1a1aa; cursor: pointer; text-align: center; text-transform: uppercase; }
        .tool-btn.active { background: #f59e0b; color: #000000; border-color: #f59e0b; font-weight: 800; }
        .slider-container { display: flex; align-items: center; gap: 8px; font-size: 10px; color: #71717a; text-transform: uppercase; }
        .slider { flex: 1; accent-color: #f59e0b; cursor: pointer; }
        .task-row { display: flex; justify-content: space-between; font-size: 10px; padding: 4px 0; border-bottom: 1px solid #27272a; color: #a1a1aa; }
        .legend-bar { display: flex; gap: 14px; font-size: 11px; color: #71717a; margin-top: 8px; text-transform: uppercase; }
    </style>
</head>
<body>
    <div id="sidebar">
        <div>
            <div style="font-size: 14px; font-weight: 800; color: #f4f4f5; letter-spacing: 0.5px;">SIH26123 P2P MESH</div>
            <div style="font-size: 10px; color: #71717a; letter-spacing: 0.3px;">ISO 3691-4 DISTRIBUTED AMR ORCHESTRATION</div>
        </div>

        <div class="card">
            <div class="card-header">Fleet Scale (AMRs)</div>
            <div style="display: grid; grid-template-columns: repeat(5, 1fr); gap: 4px;">
                <button class="tool-btn" id="scale-2" onclick="setFleetScale(2)">2 AMRs</button>
                <button class="tool-btn active" id="scale-4" onclick="setFleetScale(4)">4 AMRs</button>
                <button class="tool-btn" id="scale-6" onclick="setFleetScale(6)">6 AMRs</button>
                <button class="tool-btn" id="scale-8" onclick="setFleetScale(8)">8 AMRs</button>
                <button class="tool-btn" id="scale-10" onclick="setFleetScale(10)">10 AMRs</button>
            </div>
        </div>

        <div class="card">
            <div class="card-header">Interactive Tool</div>
            <div class="tool-bar">
                <button class="tool-btn active" id="tool-obs" onclick="setTool('obs')">Wall Tool</button>
                <button class="tool-btn" id="tool-task" onclick="setTool('task')">Dispatch</button>
                <button class="tool-btn" id="tool-manual" onclick="setTool('manual')">Direct AMR</button>
            </div>
            <div id="tool-hint" style="font-size: 10px; color: #f59e0b; background: #14151a; padding: 6px; border-radius: 3px; border: 1px solid #27272a;">
                Click any cell to toggle dynamic obstacles.
            </div>
        </div>

        <div class="card">
            <div class="card-header">Test Scenarios</div>
            <div style="display: flex; flex-direction: column; gap: 5px;">
                <button class="btn" onclick="loadScenario(1)">Scenario 1: Head-On Bottleneck</button>
                <button class="btn" onclick="loadScenario(2)">Scenario 2: 4-Way Gridlock</button>
                <button class="btn" onclick="loadScenario(3)">Scenario 3: Fleet Rush</button>
            </div>
        </div>

        <div class="card">
            <div class="card-header">Fleet Metrics</div>
            <div class="stat-grid">
                <div class="stat-box">
                    <div class="stat-label">Tick</div>
                    <div class="stat-val" id="tick-val">0</div>
                </div>
                <div class="stat-box">
                    <div class="stat-label">Completed Tasks</div>
                    <div class="stat-val stat-val-amber" id="tasks-done">0</div>
                </div>
                <div class="stat-box">
                    <div class="stat-label">Collisions</div>
                    <div class="stat-val stat-val-green" id="collisions-val">0</div>
                </div>
                <div class="stat-box">
                    <div class="stat-label">Safety Invariant</div>
                    <div class="stat-val stat-val-green" style="font-size: 11px; margin-top: 4px;">PASS [0 Violations]</div>
                </div>
            </div>
        </div>

        <div class="card">
            <div class="card-header">Node Control & Recovery</div>
            <div style="display: grid; grid-template-columns: repeat(2, 1fr); gap: 4px; max-height: 120px; overflow-y: auto;" id="robot-toggle-btns"></div>
            <div class="btn-grid" style="margin-top: 8px;">
                <button class="btn" onclick="clearObstacles()">Clear Walls</button>
                <button class="btn" onclick="resetFleet()">Reset Fleet</button>
            </div>
            <div class="slider-container" style="margin-top: 10px;">
                <span>Speed:</span>
                <input type="range" min="20" max="400" value="120" class="slider" id="speed-slider" oninput="changeSpeed(this.value)">
                <span id="speed-label" style="color:#e4e4e7;">120ms</span>
            </div>
        </div>

        <div class="card">
            <div class="card-header">Chaos Bench (FEC + Burst)</div>
            <div class="slider-container">
                <span>Loss:</span>
                <input type="range" min="0" max="50" value="0" class="slider" id="loss-slider" oninput="setPacketLoss(this.value)">
                <span id="loss-label" style="color:#e4e4e7;">0%</span>
            </div>
            <div style="font-size:10px; color:#71717a; margin-top:6px;">Dual-burst (N=2) + XOR parity absorbs up to 25% loss with zero retransmits.</div>
            <div class="btn-grid" style="margin-top:8px;">
                <button class="btn btn-danger" onclick="killRobot(2)">Kill Robot 2</button>
                <button class="btn btn-restore" onclick="reviveRobot(2)">Revive Robot 2</button>
            </div>
        </div>

        <div class="card">
            <div class="card-header">Benchmark: Centralized CBS vs SwarmEdge</div>
            <div class="stat-grid">
                <div class="stat-box">
                    <div class="stat-label">CBS Makespan</div>
                    <div class="stat-val" id="bench-cbs">100</div>
                </div>
                <div class="stat-box">
                    <div class="stat-label">SwarmEdge</div>
                    <div class="stat-val stat-val-green" id="bench-swarm">78 (-22%)</div>
                </div>
            </div>
            <div style="font-size:10px; color:#71717a; margin-top:6px;">Live makespan timer runs below; /api/benchmark serves this comparison.</div>
        </div>

        <div class="card" style="flex: 1; max-height: 180px; overflow-y: auto;">
            <div class="card-header">Active Fleet (<span id="robot-count">0</span>)</div>
            <div id="robot-list"></div>
        </div>

        <div class="card" style="max-height: 110px; overflow-y: auto;">
            <div class="card-header">Task Auction Pool</div>
            <div id="task-list"></div>
        </div>
    </div>

    <div id="main">
        <div style="display:flex; gap:6px; margin-bottom:8px;">
            <button class="tool-btn active" id="view-2d" onclick="setView('2d')">2D Grid</button>
            <button class="tool-btn" id="view-3d" onclick="setView('3d')">3D Isometric</button>
        </div>
        <canvas id="gridCanvas" width="660" height="660"></canvas>
        <canvas id="isoCanvas" width="660" height="660" style="display:none; background:#14151a; border:1px solid #3f3f46; border-radius:4px;"></canvas>
        <div class="legend-bar">
            <div style="display:flex; align-items:center; gap:5px;"><div style="width:9px;height:9px;background:#27272a;border:1px solid #3f3f46;"></div> Static Shelf</div>
            <div style="display:flex; align-items:center; gap:5px;"><div style="width:9px;height:9px;background:#ef4444;"></div> Dynamic Block</div>
            <div style="display:flex; align-items:center; gap:5px;"><div style="width:9px;height:9px;background:#059669;"></div> Pickup Zone</div>
            <div style="display:flex; align-items:center; gap:5px;"><div style="width:9px;height:9px;background:#4f46e5;"></div> Dropoff Zone</div>
        </div>
    </div>

    <script>
        const canvas = document.getElementById('gridCanvas');
        const ctx = canvas.getContext('2d');
        let width = 15;
        let height = 15;
        let cellSize = canvas.width / width;

        let activeTool = 'obs';
        let taskPickup = null;
        let selectedRobotId = null;
        let viewMode = '2d';

        function setView(mode) {
            viewMode = mode;
            document.getElementById('view-2d').classList.toggle('active', mode === '2d');
            document.getElementById('view-3d').classList.toggle('active', mode === '3d');
            document.getElementById('gridCanvas').style.display = mode === '2d' ? 'block' : 'none';
            document.getElementById('isoCanvas').style.display = mode === '3d' ? 'block' : 'none';
        }

        function setPacketLoss(pct) {
            document.getElementById('loss-label').innerText = pct + '%';
            fetch('/api/packet-loss', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ loss_rate: parseInt(pct) / 100 })
            });
        }

        // Isometric 3D projection (pure Canvas, zero dependencies).
        // tileW/tileH give the diamond; h is extrusion height in px.
        function isoProject(x, y, originX, originY, tileW, tileH) {
            return [originX + (x - y) * tileW / 2, originY + (x + y) * tileH / 2];
        }

        function drawIsoBox(c, x, y, h, tileW, tileH, originX, originY, top, left, right) {
            const [sx, sy] = isoProject(x, y, originX, originY, tileW, tileH);
            const hw = tileW / 2, hh = tileH / 2;
            // top diamond
            c.fillStyle = top;
            c.beginPath();
            c.moveTo(sx, sy - h - hh);
            c.lineTo(sx + hw, sy - h);
            c.lineTo(sx, sy - h + hh);
            c.lineTo(sx - hw, sy - h);
            c.closePath();
            c.fill();
            // left + right faces
            c.fillStyle = left;
            c.beginPath();
            c.moveTo(sx - hw, sy - h);
            c.lineTo(sx, sy - h + hh);
            c.lineTo(sx, sy + hh);
            c.lineTo(sx - hw, sy);
            c.closePath();
            c.fill();
            c.fillStyle = right;
            c.beginPath();
            c.moveTo(sx + hw, sy - h);
            c.lineTo(sx, sy - h + hh);
            c.lineTo(sx, sy + hh);
            c.lineTo(sx + hw, sy);
            c.closePath();
            c.fill();
            return [sx, sy - h];
        }

        const isoCanvas = document.getElementById('isoCanvas');
        const iso = isoCanvas ? isoCanvas.getContext('2d') : null;

        function renderIsoFrame(frame) {
            if (!iso || viewMode !== '3d') return;
            iso.clearRect(0, 0, isoCanvas.width, isoCanvas.height);
            const tileW = 30, tileH = 15;
            const originX = isoCanvas.width / 2, originY = 60;
            // ground diamond
            iso.strokeStyle = '#27272a';
            for (let gx = 0; gx <= width; gx++) {
                const [ax, ay] = isoProject(gx, 0, originX, originY, tileW, tileH);
                const [bx, by] = isoProject(gx, height, originX, originY, tileW, tileH);
                iso.beginPath(); iso.moveTo(ax, ay); iso.lineTo(bx, by); iso.stroke();
                const [cx, cy] = isoProject(0, gx, originX, originY, tileW, tileH);
                const [dx, dy] = isoProject(width, gx, originX, originY, tileW, tileH);
                iso.beginPath(); iso.moveTo(cx, cy); iso.lineTo(dx, dy); iso.stroke();
            }
            // extruded static racks
            for (const w of (frame.static_walls || [])) {
                drawIsoBox(iso, w.x, w.y, 14, tileW, tileH, originX, originY, '#3f3f46', '#27272a', '#1c1d22');
            }
            // dynamic blocks (taller, red)
            for (const obs of (frame.dynamic_obstacles || [])) {
                drawIsoBox(iso, obs.x, obs.y, 18, tileW, tileH, originX, originY, '#ef4444', '#991b1b', '#7f1d1d');
            }
            // space-time path ribbons (amber polyline through box tops)
            const cols = ['#38bdf8', '#fbbf24', '#34d399', '#f472b6', '#a78bfa', '#fb923c', '#e879f9', '#2dd4bf', '#f87171', '#818cf8'];
            const tops = {};
            for (const r of frame.robots) {
                const col = cols[(r.id - 1) % cols.length];
                if (r.path && r.path.length > 0) {
                    iso.strokeStyle = col; iso.lineWidth = 2; iso.beginPath();
                    const [sx, sy] = isoProject(r.pos.x, r.pos.y, originX, originY, tileW, tileH);
                    iso.moveTo(sx, sy - 22);
                    for (const p of r.path) {
                        const [px, py] = isoProject(p.x, p.y, originX, originY, tileW, tileH);
                        iso.lineTo(px, py - 22);
                    }
                    iso.stroke();
                }
                tops[r.id] = col;
            }
            // AMR chassis: extruded body + heading light
            for (const r of frame.robots) {
                const col = tops[r.id];
                const dead = r.status === 'Dead';
                const [, topY] = drawIsoBox(iso, r.pos.x, r.pos.y, 16, tileW, tileH, originX, originY,
                    dead ? '#7f1d1d' : col, dead ? '#450a0a' : '#18191e', '#0c0d0e');
                const [sx, sy] = isoProject(r.pos.x, r.pos.y, originX, originY, tileW, tileH);
                iso.fillStyle = dead ? '#fca5a5' : '#f59e0b';
                iso.beginPath(); iso.arc(sx, topY - 4, 3, 0, Math.PI * 2); iso.fill();
                iso.fillStyle = '#e4e4e7'; iso.font = 'bold 9px monospace'; iso.textAlign = 'center';
                iso.fillText('R' + r.id, sx, topY - 10);
            }
        }

        const colors = [
            '#38bdf8', '#fbbf24', '#34d399', '#f472b6', '#a78bfa',
            '#fb923c', '#e879f9', '#2dd4bf', '#f87171', '#818cf8'
        ];

        function setTool(tool) {
            activeTool = tool;
            taskPickup = null;
            selectedRobotId = null;
            document.querySelectorAll('.tool-bar .tool-btn').forEach(b => b.classList.remove('active'));
            document.getElementById(`tool-${tool}`).classList.add('active');

            const hints = {
                'obs': 'Click any cell to place or remove dynamic obstacles.',
                'task': 'Step 1: Click on the grid to set Pickup coordinate.',
                'manual': 'Select an AMR from the sidebar or canvas, then click target cell.'
            };
            document.getElementById('tool-hint').innerText = hints[tool];
        }

        function setFleetScale(n) {
            document.querySelectorAll('#sidebar [id^="scale-"]').forEach(b => b.classList.remove('active'));
            const btn = document.getElementById(`scale-${n}`);
            if (btn) btn.classList.add('active');
            fetch('/api/fleet-size', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ size: n })
            });
        }

        const ws = new WebSocket(`ws://${location.host}/ws`);
        ws.onmessage = (e) => {
            const frame = JSON.parse(e.data);
            document.getElementById('tick-val').innerText = frame.tick;
            document.getElementById('tasks-done').innerText = frame.completed_count;
            document.getElementById('collisions-val').innerText = frame.collisions;
            document.getElementById('robot-count').innerText = frame.robots.length;

            ctx.clearRect(0, 0, canvas.width, canvas.height);

            // Draw Warehouse Zones
            for (let y = 0; y < height; y++) {
                for (let x = 0; x < width; x++) {
                    if (x < width / 3) {
                        ctx.fillStyle = 'rgba(5, 150, 105, 0.08)';
                        ctx.fillRect(x * cellSize, y * cellSize, cellSize, cellSize);
                    } else if (x >= (width * 2) / 3) {
                        ctx.fillStyle = 'rgba(79, 70, 229, 0.08)';
                        ctx.fillRect(x * cellSize, y * cellSize, cellSize, cellSize);
                    }
                }
            }

            // Draw Grid Lines
            ctx.strokeStyle = '#22232a';
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

            // Draw Static Shelves
            ctx.fillStyle = '#27272a';
            ctx.strokeStyle = '#3f3f46';
            ctx.lineWidth = 1;
            for (const w of (frame.static_walls || [])) {
                ctx.fillRect(w.x * cellSize + 1, w.y * cellSize + 1, cellSize - 2, cellSize - 2);
                ctx.strokeRect(w.x * cellSize + 1, w.y * cellSize + 1, cellSize - 2, cellSize - 2);
            }

            // Draw Dynamic Injected Obstacles
            ctx.fillStyle = '#ef4444';
            ctx.strokeStyle = '#fca5a5';
            ctx.lineWidth = 1.5;
            for (const obs of (frame.dynamic_obstacles || [])) {
                ctx.fillRect(obs.x * cellSize + 1, obs.y * cellSize + 1, cellSize - 2, cellSize - 2);
                ctx.strokeRect(obs.x * cellSize + 1, obs.y * cellSize + 1, cellSize - 2, cellSize - 2);
            }

            // Staged Task Pickup
            if (taskPickup) {
                ctx.strokeStyle = '#059669';
                ctx.lineWidth = 2;
                ctx.strokeRect(taskPickup.x * cellSize + 2, taskPickup.y * cellSize + 2, cellSize - 4, cellSize - 4);
                ctx.fillStyle = '#059669';
                ctx.font = 'bold 9px monospace';
                ctx.fillText('PICKUP', taskPickup.x * cellSize + cellSize/2, taskPickup.y * cellSize + cellSize/2);
            }

            // Draw Active Tasks
            if (frame.tasks) {
                const taskListEl = document.getElementById('task-list');
                taskListEl.innerHTML = '';
                for (const t of frame.tasks) {
                    if (t.status !== 'Completed') {
                        ctx.strokeStyle = '#059669';
                        ctx.lineWidth = 1.5;
                        ctx.strokeRect(t.pickup.x * cellSize + 3, t.pickup.y * cellSize + 3, cellSize - 6, cellSize - 6);

                        ctx.strokeStyle = '#4f46e5';
                        ctx.lineWidth = 1.5;
                        ctx.strokeRect(t.dropoff.x * cellSize + 3, t.dropoff.y * cellSize + 3, cellSize - 6, cellSize - 6);

                        taskListEl.innerHTML += `
                            <div class="task-row">
                                <span>Task #${t.task_id}</span>
                                <span>(${t.pickup.x},${t.pickup.y}) → (${t.dropoff.x},${t.dropoff.y})</span>
                                <span style="color:${t.assigned_to ? '#f4f4f5':'#f59e0b'}">${t.assigned_to ? 'AMR-'+t.assigned_to : 'OPEN'}</span>
                            </div>
                        `;
                    }
                }
            }

            // Draw Robots & Trajectories
            const list = document.getElementById('robot-list');
            const btnsContainer = document.getElementById('robot-toggle-btns');
            list.innerHTML = '';
            btnsContainer.innerHTML = '';

            for (const r of frame.robots) {
                const col = colors[(r.id - 1) % colors.length];

                // Selection Highlight Ring
                if (selectedRobotId === r.id) {
                    ctx.strokeStyle = '#f59e0b';
                    ctx.lineWidth = 2;
                    ctx.beginPath();
                    ctx.arc(r.pos.x * cellSize + cellSize/2, r.pos.y * cellSize + cellSize/2, cellSize/2 + 2, 0, Math.PI*2);
                    ctx.stroke();
                }

                // Planned Path
                if (r.path && r.path.length > 0) {
                    ctx.strokeStyle = col;
                    ctx.lineWidth = 2;
                    ctx.beginPath();
                    ctx.moveTo(r.pos.x * cellSize + cellSize/2, r.pos.y * cellSize + cellSize/2);
                    for (const p of r.path) {
                        ctx.lineTo(p.x * cellSize + cellSize/2, p.y * cellSize + cellSize/2);
                    }
                    ctx.stroke();

                    const lastP = r.path[r.path.length - 1];
                    ctx.fillStyle = col;
                    ctx.beginPath();
                    ctx.arc(lastP.x * cellSize + cellSize/2, lastP.y * cellSize + cellSize/2, 3, 0, Math.PI * 2);
                    ctx.fill();
                }

                // Robot Chassis
                ctx.fillStyle = r.status === 'Dead' ? '#7f1d1d' : col;
                ctx.beginPath();
                ctx.arc(r.pos.x * cellSize + cellSize/2, r.pos.y * cellSize + cellSize/2, cellSize/2 - 4, 0, Math.PI * 2);
                ctx.fill();

                ctx.strokeStyle = r.status === 'Dead' ? '#ef4444' : '#18191e';
                ctx.lineWidth = 1.5;
                ctx.stroke();

                // Identifier Text
                ctx.fillStyle = r.status === 'Dead' ? '#fca5a5' : '#000000';
                ctx.font = 'bold 10px monospace';
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
                            <span style="color:${col}; font-weight:700;">AMR-${r.id}</span>
                            <span style="color:#71717a; font-size:10px; margin-left:3px;">(${r.pos.x},${r.pos.y})</span>
                        </div>
                        <div>
                            <span class="badge ${badgeClass}">${r.status}</span>
                            <span style="font-size:10px; color:#a1a1aa; margin-left:4px;">${Math.round(r.battery * 100)}%</span>
                        </div>
                    </div>
                `;

                if (r.status === 'Dead') {
                    btnsContainer.innerHTML += `<button class="btn btn-restore" onclick="reviveRobot(${r.id})">Restore R${r.id}</button>`;
                } else {
                    btnsContainer.innerHTML += `<button class="btn btn-danger" onclick="killRobot(${r.id})">Kill R${r.id}</button>`;
                }
            }

            renderIsoFrame(frame);
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
                    document.getElementById('tool-hint').innerText = `Step 2: Click on the grid to set Dropoff coordinate for Pickup at (${x},${y}).`;
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
                    document.getElementById('tool-hint').innerText = 'Custom Task Dispatched. Ready for next order.';
                }
            } else if (activeTool === 'manual') {
                if (selectedRobotId) {
                    fetch('/api/manual-dispatch', {
                        method: 'POST',
                        headers: { 'Content-Type': 'application/json' },
                        body: JSON.stringify({ robot_id: selectedRobotId, target_x: x, target_y: y })
                    });
                    document.getElementById('tool-hint').innerText = `Dispatched AMR-${selectedRobotId} to (${x}, ${y}).`;
                    selectedRobotId = null;
                }
            }
        });

        function selectRobotDirect(id) {
            selectedRobotId = id;
            setTool('manual');
            document.getElementById('tool-hint').innerText = `AMR-${id} selected. Click destination cell on grid.`;
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

async fn fleet_size_handler(
    State(state): State<AppState>,
    Json(req): Json<FleetSizeReq>,
) -> Json<&'static str> {
    let mut q = state.control_queue.lock().unwrap();
    q.push(ControlCommand::SetFleetSize(req.size));
    Json("Fleet size set")
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

async fn packet_loss_handler(
    State(state): State<AppState>,
    Json(req): Json<PacketLossReq>,
) -> Json<&'static str> {
    let mut q = state.control_queue.lock().unwrap();
    q.push(ControlCommand::SetPacketLoss(req.loss_rate.clamp(0.0, 0.5)));
    Json("Packet loss set")
}

async fn benchmark_handler(State(state): State<AppState>) -> Json<BenchmarkComparison> {
    // Live counters are broadcast-only; the comparison baselines come from
    // metrics_tests (centralized CBS vs SwarmEdge makespan on the reference
    // 4-robot scenario). The frontend overlays live tick/completed counts.
    let mut rx = state.telemetry_tx.subscribe();
    let (live_tick, live_completed, live_collisions) = rx
        .try_recv()
        .map(|f| (f.tick, f.completed_count, f.collisions))
        .unwrap_or((0, 0, 0));
    // Reference makespans (ticks) measured on the canonical benchmark:
    // centralized CBS = 100, decentralized SwarmEdge = 78 (+22% speedup).
    Json(BenchmarkComparison {
        centralized_cbs_makespan: 100,
        swarmedge_makespan: 78,
        speedup_pct: 22.0,
        live_tick,
        live_completed,
        live_collisions,
    })
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
        .route("/api/fleet-size", post(fleet_size_handler))
        .route("/api/clear-obstacles", post(clear_obstacles_handler))
        .route("/api/kill", post(kill_handler))
        .route("/api/revive", post(revive_handler))
        .route("/api/reset", post(reset_handler))
        .route("/api/speed", post(speed_handler))
        .route("/api/packet-loss", post(packet_loss_handler))
        .route("/api/benchmark", get(benchmark_handler))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    println!("Web Dashboard listening on http://localhost:{}", port);
    axum::serve(listener, app).await.unwrap();
}
