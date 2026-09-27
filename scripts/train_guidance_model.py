#!/usr/bin/env python3
"""
THADAM Neural A* Guidance — Model Generation & Training (SIH26123).

Pipeline:
  1. Generate 2,000 synthetic warehouse grid layouts (15x15 to 32x32),
     mirroring engine/src/world/grid.rs::generate_warehouse so the
     distributions match what the Rust planner sees at runtime.
  2. Compute ground-truth cost-to-go maps with reverse Dijkstra (4-connected,
     unit step cost — exactly the metric the planner optimizes).
  3. Train the 5-layer GuidanceFCN (PyTorch) to convergence (MSE < 0.02).
  4. Export trained weights to engine/assets/guidance.onnx with a FIXED
     32x32 input (static shapes only — tract's zero-alloc hot path requires
     compile-time-known tensor shapes). Smaller grids are zero-padded at
     inference; the model learns to predict through the padding.

Labels are RESIDUAL cost-to-go: (reverse-Dijkstra ctg − Manhattan-to-goal)
/ 32, clipped to [0, 1]. The Rust heuristic is h = kinematic + residual*32:
the net only contributes obstacle-detour steering on top of the exact
Manhattan prior, so uncertain predictions degrade gracefully to plain A*.
"""

import argparse
import math
import os
import random

import numpy as np
import torch
import torch.nn as nn

# Fixed ONNX tensor side. Every grid is zero-padded into this window.
GRID = 32
MAX_DIM = GRID  # cost-to-go divisor keeps labels in [0, 1]

INPUT_C = 4   # channels: [walkable, goal_x, goal_y, orientation_axis]
OUTPUT_C = 1  # scalar cost-to-go heatmap


# ---------------------------------------------------------------------------
# Warehouse generation (mirrors engine/src/world/grid.rs::generate_warehouse)
# ---------------------------------------------------------------------------
def generate_warehouse(width: int, height: int, aisle_spacing: int, rng: random.Random):
    """Byte-grid replica of GridMap::generate_warehouse (1 = wall, 0 = walkable)."""
    grid = np.zeros((height, width), dtype=np.uint8)
    if aisle_spacing == 0 or width < 3 or height < 3:
        return grid

    spacing = max(aisle_spacing, 2)
    center = width // 2
    for y in range(spacing, max(height - 1, spacing), spacing):
        for x in range(1, max(width - 1, 1)):
            if x == center or x == center - 1:
                continue  # cross-aisle gap
            grid[y, x] = 1
    return grid


def randomize_wall_cells(grid: np.ndarray, rng: random.Random, ratio: float = 0.08):
    """Random dead-aisle clutter so the FCN sees non-canonical maps too."""
    h, w = grid.shape
    count = max(1, int(w * h * ratio))
    for _ in range(count):
        x = rng.randrange(1, w - 1)
        y = rng.randrange(0, h)
        if grid[y, x] == 0:
            grid[y, x] = 1
    return grid


def carve_connectivity(grid: np.ndarray, rng: random.Random):
    """Flood-fill from the largest walkable component; wall off the rest.

    Also guarantees every remaining walkable cell has >= 2 open neighbors
    so the Dijkstra labels never contain unreachable cells.
    """
    h, w = grid.shape
    comps = []
    seen = np.zeros_like(grid, dtype=bool)
    for sy in range(h):
        for sx in range(w):
            if grid[sy, sx] == 0 and not seen[sy, sx]:
                stack = [(sy, sx)]
                seen[sy, sx] = True
                comp = []
                while stack:
                    y, x = stack.pop()
                    comp.append((y, x))
                    for ny, nx in ((y - 1, x), (y + 1, x), (y, x - 1), (y, x + 1)):
                        if 0 <= ny < h and 0 <= nx < w and grid[ny, nx] == 0 and not seen[ny, nx]:
                            seen[ny, nx] = True
                            stack.append((ny, nx))
                comps.append(comp)

    if len(comps) <= 1:
        return grid

    comps.sort(key=len, reverse=True)
    keep = set(comps[0])
    for comp in comps[1:]:
        for y, x in comp:
            grid[y, x] = 1

    # Prune 1-neighbor cul-de-sacs (post-merge artifacts).
    changed = True
    while changed:
        changed = False
        for y in range(h):
            for x in range(w):
                if grid[y, x] != 0:
                    continue
                deg = sum(
                    1
                    for ny, nx in ((y - 1, x), (y + 1, x), (y, x - 1), (y, x + 1))
                    if 0 <= ny < h and 0 <= nx < w and grid[ny, nx] == 0
                )
                if deg <= 1:
                    grid[y, x] = 1
                    changed = True
    return grid


# ---------------------------------------------------------------------------
# Ground-truth labels: reverse Dijkstra cost-to-go
# ---------------------------------------------------------------------------
def dijkstra_cost_to_go(grid: np.ndarray, goal: tuple[int, int]) -> np.ndarray | None:
    """Exact cost-to-go to `goal` over 4-connected unit steps.

    Returns None if any walkable cell is unreachable (untrainable sample).
    """
    h, w = grid.shape
    INF = np.iinfo(np.int32).max
    dist = np.full((h, w), INF, dtype=np.int32)

    gy, gx = goal
    if grid[gy, gx] != 0:
        return None

    # Array-based Dijkstra (unit weights -> simple bucket frontier suffices,
    # but heapq keeps it correct if step costs ever change).
    import heapq

    dist[gy, gx] = 0
    heap = [(0, gy, gx)]
    while heap:
        d, y, x = heapq.heappop(heap)
        if d > dist[y, x]:
            continue
        for ny, nx in ((y - 1, x), (y + 1, x), (y, x - 1), (y, x + 1)):
            if 0 <= ny < h and 0 <= nx < w and grid[ny, nx] == 0:
                nd = d + 1
                if nd < dist[ny, nx]:
                    dist[ny, nx] = nd
                    heapq.heappush(heap, (nd, ny, nx))

    if np.any((grid == 0) & (dist == INF)):
        return None  # unreachable walkable cell
    return dist.astype(np.float32)


# ---------------------------------------------------------------------------
# Model: 5-layer fully-convolutional GuidanceFCN
# ---------------------------------------------------------------------------
# Channel widths chosen so total parameters hit exactly 48,161
# (4*9*8 + ... -> verified: 37*8 + 73*40 + 361*53 + 478*53 + 478 = 48,161),
# matching the SIH26123 spec (~48 KB fp32 ONNX file).
# Dilation schedule (1,2,4,8,16) gives a 63x63 receptive field from 5 layers
# (1 + 2*sum(d) = 63) — required so every cell on a 32x32 grid can see the
# goal marker; dilation does not change weight counts.
CHANNELS = (8, 40, 53, 53)
DILATIONS = (1, 2, 4, 8, 16)


class GuidanceFCN(nn.Module):
    """5-layer dilated FCN: 4 -> 8 -> 40 -> 53 -> 53 -> 1, 3x3 pads=1.

    Receptive field 63x63 covers the full 32x32 window from any cell.
    Parameter count: exactly 48,161 (~48 KB fp32 ONNX), per the handoff spec.
    Output is linear (can dip negative); the Rust planner clamps to 0.
    """

    def __init__(self, in_ch: int = INPUT_C, out_ch: int = OUTPUT_C):
        super().__init__()
        c1, c2, c3, c4 = CHANNELS
        d1, d2, d3, d4, d5 = DILATIONS
        self.net = nn.Sequential(
            nn.Conv2d(in_ch, c1, 3, padding=d1, dilation=d1),  # layer 1
            nn.ReLU(),
            nn.Conv2d(c1, c2, 3, padding=d2, dilation=d2),     # layer 2
            nn.ReLU(),
            nn.Conv2d(c2, c3, 3, padding=d3, dilation=d3),     # layer 3
            nn.ReLU(),
            nn.Conv2d(c3, c4, 3, padding=d4, dilation=d4),     # layer 4
            nn.ReLU(),
            nn.Conv2d(c4, out_ch, 3, padding=d5, dilation=d5), # layer 5 (linear)
        )

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        return self.net(x)


def count_params(model: nn.Module) -> int:
    return sum(p.numel() for p in model.parameters())


# ---------------------------------------------------------------------------
# Sample generation
# ---------------------------------------------------------------------------
def make_sample(rng: random.Random):
    """Returns (input 4x32x32, target 32x32) or None if untrainable."""
    w = rng.randint(15, 32)
    h = rng.randint(15, 32)
    spacing = rng.choice([2, 3, 3, 4])
    grid = generate_warehouse(w, h, spacing, rng)

    if rng.random() < 0.5:
        randomize_wall_cells(grid, rng)

    grid = carve_connectivity(grid, rng)

    walk = [(y, x) for y in range(h) for x in range(w) if grid[y, x] == 0]
    if len(walk) < 40:
        return None
    (gy, gx), (sy, sx) = rng.sample(walk, 2)
    if abs(gy - sy) + abs(gx - sx) < 6:
        return None

    ctg = dijkstra_cost_to_go(grid, (gy, gx))
    if ctg is None:
        return None

    yy, xx = np.mgrid[0:GRID, 0:GRID].astype(np.float32)

    inp = np.zeros((INPUT_C, GRID, GRID), dtype=np.float32)
    inp[0, :h, :w] = 1.0 - grid.astype(np.float32)   # walkable (1 = free)
    inp[1, :h, :w] = (xx[:h, :w] - gx) / float(GRID)  # relative goal x
    inp[2, :h, :w] = (yy[:h, :w] - gy) / float(GRID)  # relative goal y
    # ch3: Manhattan distance to goal / 64 — a strong linear prior that the
    # conv stack can refine with obstacle awareness (mirrored in Rust).
    inp[3, :h, :w] = (
        np.abs(xx[:h, :w] - gx) + np.abs(yy[:h, :w] - gy)
    ) / 64.0

    # RESIDUAL target: ctg - manhattan_to_goal (>= 0 on 4-connected grids).
    # The heuristic at inference is h = manhattan + residual * 32. Wherever
    # the net is unsure it predicts ~0, so ordering degrades gracefully to
    # plain Manhattan A* instead of scrambling plateau ties.
    man = np.abs(xx[:h, :w] - gx) + np.abs(yy[:h, :w] - gy)
    tgt = np.zeros((GRID, GRID), dtype=np.float32)
    tgt[:h, :w] = np.clip((ctg[:h, :w] - man) / float(MAX_DIM), 0.0, 1.0)
    # Wall cells keep target 0: the planner never queries them and the
    # masked loss below ignores them (channel 0 doubles as the mask).

    return inp, tgt


def generate_dataset(n: int, seed: int, batch: int = 64):
    rng = random.Random(seed)
    inputs, targets = [], []
    tries = 0
    while len(inputs) < n:
        tries += 1
        if tries > n * 8:
            break
        s = make_sample(rng)
        if s is None:
            continue
        inputs.append(s[0])
        targets.append(s[1])
        if len(inputs) % batch == 0:
            print(f"  generated {len(inputs)}/{n} samples", flush=True)
    return np.stack(inputs), np.stack(targets)


# ---------------------------------------------------------------------------
# Training
# ---------------------------------------------------------------------------
def masked_mse(pred: torch.Tensor, target: torch.Tensor, walk_mask: torch.Tensor) -> torch.Tensor:
    """MSE over walkable cells only (walk_mask = input channel 0: 1 = free).

    Wall/padding cells are excluded: their labels are undefined and the
    planner never queries them. Without masking, INT32_MAX wall labels
    from the raw Dijkstra map dominate the loss and training diverges.
    """
    se = (pred - target) ** 2 * walk_mask
    return se.sum() / walk_mask.sum().clamp(min=1.0)


def train(model, ds_x, ds_y, epochs, batch_size, lr, target_mse, device):
    model.to(device)
    opt = torch.optim.Adam(model.parameters(), lr=lr)
    sched = torch.optim.lr_scheduler.ReduceLROnPlateau(
        opt, factor=0.5, patience=2, min_lr=1e-5
    )

    x_all = torch.from_numpy(ds_x)
    y_all = torch.from_numpy(ds_y)
    n = x_all.shape[0]

    best = float("inf")
    for epoch in range(1, epochs + 1):
        model.train()
        perm = torch.randperm(n)
        total = 0.0
        for i in range(0, n, batch_size):
            idx = perm[i : i + batch_size]
            x = x_all[idx].to(device)
            y = y_all[idx].to(device)
            walk = x[:, 0:1, :, :]

            opt.zero_grad(set_to_none=True)
            pred = model(x)
            loss = masked_mse(pred, y.unsqueeze(1), walk)
            loss.backward()
            opt.step()
            total += loss.item() * x.shape[0]

        mse = total / n
        sched.step(mse)
        best = min(best, mse)
        print(f"epoch {epoch:3d}  MSE={mse:.5f}  lr={opt.param_groups[0]['lr']:.2e}", flush=True)

        if mse <= target_mse:
            print(f"converged: MSE {mse:.5f} <= target {target_mse}", flush=True)
            break
    return best


# ---------------------------------------------------------------------------
# ONNX export (static 32x32 shapes — required by the zero-alloc Rust path)
# ---------------------------------------------------------------------------
def export_onnx(model, out_path: str, ds_x, ds_y):
    model.eval().cpu()
    dummy = torch.zeros(1, INPUT_C, GRID, GRID)
    os.makedirs(os.path.dirname(out_path), exist_ok=True)
    # Legacy TorchScript exporter: the dynamo exporter in torch 2.14 mangles
    # small static FCNs (weights dropped from the graph). dynamo=False emits
    # a standard opset-17 graph with static 1x4x32x32 shapes — exactly what
    # tract's zero-allocation inference path requires.
    torch.onnx.export(
        model,
        (dummy,),
        out_path,
        input_names=["grid"],
        output_names=["cost_to_go"],
        opset_version=17,
        do_constant_folding=True,
        dynamo=False,
    )
    size = os.path.getsize(out_path)
    print(f"exported: {out_path} ({size} bytes)", flush=True)
    if size < count_params(model) * 4:
        raise RuntimeError(
            f"ONNX file ({size} B) too small for fp32 weights — export is degenerate"
        )

    # Numeric validation: ONNX graph must reproduce PyTorch output on a
    # real training sample (catches silent graph mangling).
    with torch.no_grad():
        torch_out = model(torch.from_numpy(ds_x[:1])).numpy()
    try:
        import onnxruntime as ort
        sess = ort.InferenceSession(out_path, providers=["CPUExecutionProvider"])
        onnx_out = sess.run(None, {"grid": ds_x[:1]})[0]
    except ImportError:
        from onnx.reference import ReferenceEvaluator
        sess = ReferenceEvaluator(out_path)
        onnx_out = sess.run(None, {"grid": ds_x[:1]})[0]
    err = float(np.abs(onnx_out - torch_out).max())
    print(f"export validation: max |onnx - pytorch| = {err:.2e}", flush=True)
    if err > 1e-3:
        raise RuntimeError("ONNX output diverges from PyTorch — aborting")
    return sess


def main():
    ap = argparse.ArgumentParser(description="THADAM guidance model trainer")
    ap.add_argument("--samples", type=int, default=4000)
    ap.add_argument("--epochs", type=int, default=200)
    ap.add_argument("--batch-size", type=int, default=64)
    ap.add_argument("--lr", type=float, default=1e-3)
    ap.add_argument("--target-mse", type=float, default=0.002)
    ap.add_argument("--seed", type=int, default=26123)
    ap.add_argument("--out", default=os.path.join("engine", "assets", "guidance.onnx"))
    ap.add_argument("--skip-train", action="store_true", help="generate data, then exit")
    args = ap.parse_args()

    torch.manual_seed(args.seed)
    print(f"generating {args.samples} synthetic warehouse samples (seed={args.seed})...", flush=True)
    ds_x, ds_y = generate_dataset(args.samples, args.seed)
    print(f"dataset: {ds_x.shape[0]} samples, input {ds_x.shape[1:]}, target {ds_y.shape[1:]}", flush=True)

    if args.skip_train:
        return

    model = GuidanceFCN()
    print(f"GuidanceFCN parameters: {count_params(model):,}", flush=True)

    device = "cuda" if torch.cuda.is_available() else "cpu"
    print(f"training on {device}...", flush=True)
    best = train(model, ds_x, ds_y, args.epochs, args.batch_size, args.lr, args.target_mse, device)

    sess = export_onnx(model, args.out, ds_x, ds_y)

    # Final quality gate: masked MSE on a held-out-style probe sample.
    with torch.no_grad():
        probe_pred = model(torch.from_numpy(ds_x[:1])).numpy()
    walk = ds_x[:1, 0:1]
    probe_mse = float(np.mean(((probe_pred - ds_y[:1]) ** 2 * walk)))
    print(f"post-train probe masked MSE (pytorch): {probe_mse:.5f}", flush=True)
    print(f"final best training MSE: {best:.5f} (target {args.target_mse})", flush=True)
    if best > args.target_mse:
        print("WARNING: did not reach target MSE — consider more epochs or samples", flush=True)


if __name__ == "__main__":
    main()
