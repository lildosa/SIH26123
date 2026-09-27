#!/usr/bin/env python3
"""Aggregates `thadam batch` output into mean ± std tables per (size, robots, config).

Completion rate is reported separately from makespan: runs that hit the tick
cap are excluded from makespan stats (a 600-tick timeout is not a makespan).
"""

import sys
from collections import defaultdict

import numpy as np


def main():
    rows = []
    for line in sys.stdin:
        parts = line.split()
        if len(parts) != 11 or parts[0] == "seed":
            continue  # header or malformed
        try:
            rows.append(
                dict(
                    seed=int(parts[0]),
                    robots=int(parts[1]),
                    size=int(parts[2]),
                    config=parts[3],
                    done=int(parts[4]),
                    total=int(parts[5]),
                    makespan=int(parts[6]),
                    collisions=int(parts[7]),
                    expansions=int(parts[8]),
                    learned=int(parts[9]),
                    fallbacks=int(parts[10]),
                )
            )
        except ValueError:
            continue

    if not rows:
        print("no valid batch rows on stdin", file=sys.stderr)
        sys.exit(1)

    groups = defaultdict(list)
    for r in rows:
        groups[(r["size"], r["robots"], r["config"])].append(r)

    print(f"{'size':>5} {'robots':>6} {'config':>8} | {'compl%':>7} {'makespan*':>11} "
          f"{'collisions':>10} {'expansions':>12} {'n':>3}")
    print("-" * 78)
    for (size, robots, config), rs in sorted(groups.items()):
        n = len(rs)
        compl = np.array([r["done"] / r["total"] * 100 for r in rs])
        done_runs = [r["makespan"] for r in rs if r["done"] >= r["total"]]
        cols = np.array([r["collisions"] for r in rs])
        exp = np.array([r["expansions"] for r in rs], dtype=float)

        def fmt(a, spec="{:.1f}"):
            if len(a) == 0:
                return "n/a"
            return f"{spec.format(a.mean())}±{spec.format(a.std(ddof=1))}" if len(a) > 1 else spec.format(a.mean())

        compl_s = fmt(compl)
        makespan_s = fmt(np.array(done_runs, dtype=float)) if done_runs else "no completion"
        coll_s = fmt(cols, "{:.1f}")
        exp_s = fmt(exp, "{:.0f}")
        print(f"{size:>5} {robots:>6} {config:>8} | {compl_s:>7} {makespan_s:>11} "
              f"{coll_s:>10} {exp_s:>12} {n:>3}")

    print("\n*makespan stats exclude runs that hit the tick cap (incomplete).")
    print(" values are mean±std over seeds.")


if __name__ == "__main__":
    main()
