#!/usr/bin/env python3
"""
SIH26123 - Distributed AMR Fleet Coordination Demo (Matplotlib Animated Visualizer)
Connects to the live WebSocket stream or simulates a multi-robot warehouse fleet.
"""

import json
import asyncio
import threading
import matplotlib.pyplot as plt
import matplotlib.animation as animation
import matplotlib.patches as patches
import urllib.request

try:
    import websockets
    HAS_WEBSOCKETS = True
except ImportError:
    HAS_WEBSOCKETS = False

fig, ax = plt.subplots(figsize=(8, 8))
fig.patch.set_facecolor('#0f172a')
ax.set_facecolor('#020617')

WIDTH, HEIGHT = 15, 15
current_frame = {
    "tick": 0,
    "robots": [],
    "obstacles": [],
    "tasks": [],
    "completed_count": 0,
    "collisions": 0
}

COLORS = ['#38bdf8', '#4ade80', '#fbbf24', '#f472b6', '#a78bfa', '#fb7185']

async def ws_client():
    global current_frame
    uri = "ws://localhost:3000/ws"
    while True:
        try:
            async with websockets.connect(uri) as websocket:
                print("[Matplotlib Visualizer] Connected to WebSocket at ws://localhost:3000/ws")
                while True:
                    msg = await websocket.recv()
                    data = json.loads(msg)
                    current_frame = data
        except Exception as e:
            await asyncio.sleep(1.0)

def start_ws_thread():
    if HAS_WEBSOCKETS:
        loop = asyncio.new_event_loop()
        threading.Thread(target=lambda: loop.run_until_complete(ws_client()), daemon=True).start()

def update(frame_num):
    ax.clear()
    ax.set_facecolor('#020617')
    ax.set_xlim(-0.5, WIDTH - 0.5)
    ax.set_ylim(-0.5, HEIGHT - 0.5)
    ax.invert_yaxis()

    # Draw grid
    ax.set_xticks(range(WIDTH))
    ax.set_yticks(range(HEIGHT))
    ax.grid(color='#1e293b', linestyle='-', linewidth=1)

    # Draw Obstacles
    for obs in current_frame.get("obstacles", []):
        rect = patches.Rectangle(
            (obs["x"] - 0.45, obs["y"] - 0.45), 0.9, 0.9,
            facecolor='#334155', edgecolor='#475569', linewidth=1.5
        )
        ax.add_patch(rect)

    # Draw Tasks
    for t in current_frame.get("tasks", []):
        if t.get("status") in ["Open", "Assigned", "InProgress"]:
            p = t["pickup"]
            d = t["dropoff"]
            # Pickup (Green)
            ax.add_patch(patches.Rectangle((p["x"] - 0.35, p["y"] - 0.35), 0.7, 0.7,
                                           facecolor='none', edgecolor='#10b981', linewidth=2, linestyle='--'))
            # Dropoff (Purple)
            ax.add_patch(patches.Rectangle((d["x"] - 0.35, d["y"] - 0.35), 0.7, 0.7,
                                           facecolor='none', edgecolor='#a855f7', linewidth=2, linestyle='--'))

    # Draw Robots & Paths
    for r in current_frame.get("robots", []):
        rid = r["id"]
        col = COLORS[(rid - 1) % len(COLORS)]
        pos = r["pos"]

        # Path trace
        path = r.get("path", [])
        if path:
            xs = [pos["x"]] + [p["x"] for p in path]
            ys = [pos["y"]] + [p["y"] for p in path]
            ax.plot(xs, ys, color=col, linewidth=2, alpha=0.8)

        # Robot Circle
        circle = patches.Circle((pos["x"], pos["y"]), 0.38, facecolor=col, edgecolor='#ffffff', linewidth=1.5)
        ax.add_patch(circle)
        ax.text(pos["x"], pos["y"], f"R{rid}", color='#000000', fontsize=9, fontweight='bold', ha='center', va='center')

    # Title & Stats
    ax.set_title(
        f"SIH26123 AMR Mesh | Tick: {current_frame['tick']} | Tasks Done: {current_frame.get('completed_count', 0)} | Collisions: {current_frame.get('collisions', 0)}",
        color='#38bdf8', fontsize=12, fontweight='bold', pad=12
    )

def on_click(event):
    if event.xdata is not None and event.ydata is not None:
        x = int(round(event.xdata))
        y = int(round(event.ydata))
        if 0 <= x < WIDTH and 0 <= y < HEIGHT:
            try:
                req = urllib.request.Request(
                    "http://localhost:3000/api/obstacle",
                    data=json.dumps({"x": x, "y": y}).encode('utf-8'),
                    headers={"Content-Type": "application/json"}
                )
                urllib.request.urlopen(req)
                print(f"[Matplotlib] Toggled obstacle at ({x}, {y})")
            except Exception as ex:
                print(f"[Matplotlib] Error toggling obstacle: {ex}")

fig.canvas.mpl_connect('button_press_event', on_click)

if __name__ == '__main__':
    start_ws_thread()
    ani = animation.FuncAnimation(fig, update, interval=100, cache_frame_data=False)
    plt.tight_layout()
    plt.show()
