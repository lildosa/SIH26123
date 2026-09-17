#!/usr/bin/env python3
"""
Generates the high-level architecture diagram following the Diagram Design skill:
- Self-contained HTML file: docs/diagrams/high_level_architecture.html
- Standalone SVG: docs/diagrams/high_level_architecture.svg
- High-res PNG rendering: docs/diagrams/high_level_architecture.png
"""

import os
import subprocess

HTML_CONTENT = """<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>EdgeBot · High-Level Architecture</title>
  <link href="https://fonts.googleapis.com/css2?family=Instrument+Serif:ital@0;1&family=Geist:wght@400;500;600&family=Geist+Mono:wght@400;500;600&display=swap" rel="stylesheet">
  <style>
    *, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }
    :root {
      --color-paper:   #f5f5f5;
      --color-paper-2: #ececec;
      --color-ink:     #2d3142;
      --color-muted:   #4f5d75;
      --color-soft:    #7a8399;
      --color-accent:  #eb6c36;
      --color-accent-tint: rgba(235, 108, 54, 0.08);
      --color-link:    #2e5aa8;
      --color-rule:    rgba(45, 49, 66, 0.12);
      --font-sans:     'Geist', system-ui, -apple-system, sans-serif;
      --font-serif:    'Instrument Serif', Georgia, serif;
      --font-mono:     'Geist Mono', ui-monospace, monospace;
    }

    body {
      font-family: var(--font-sans);
      background: var(--color-paper);
      color: var(--color-ink);
      min-height: 100vh;
      display: flex;
      flex-direction: column;
      align-items: center;
      padding: 3rem 2rem;
    }

    .frame {
      max-width: 1160px;
      width: 100%;
    }

    .eyebrow {
      font-family: var(--font-mono);
      font-size: 0.72rem;
      font-weight: 600;
      letter-spacing: 0.18em;
      text-transform: uppercase;
      color: var(--color-muted);
      margin-bottom: 0.5rem;
    }

    h1 {
      font-family: var(--font-serif);
      font-size: clamp(2rem, 3vw + 1rem, 2.75rem);
      font-weight: 400;
      letter-spacing: -0.02em;
      line-height: 1.1;
      color: var(--color-ink);
      margin-bottom: 0.5rem;
    }

    .subtitle {
      font-size: 0.95rem;
      color: var(--color-muted);
      margin-bottom: 2rem;
      max-width: 760px;
      line-height: 1.5;
    }

    .diagram-container {
      background: #ffffff;
      border: 1px solid var(--color-rule);
      border-radius: 8px;
      padding: 1.5rem;
      margin-bottom: 2.5rem;
      overflow-x: auto;
    }

    svg {
      width: 100%;
      min-width: 960px;
      display: block;
    }

    .card-grid {
      display: grid;
      grid-template-columns: 1.2fr 1fr 0.9fr;
      gap: 1.25rem;
      margin-bottom: 2.5rem;
    }

    .card {
      background: #ffffff;
      border: 1px solid var(--color-rule);
      border-radius: 6px;
      padding: 1.25rem;
    }

    .card-eyebrow {
      font-family: var(--font-mono);
      font-size: 0.65rem;
      font-weight: 600;
      letter-spacing: 0.16em;
      text-transform: uppercase;
      color: var(--color-soft);
      margin-bottom: 0.5rem;
    }

    .card-header {
      display: flex;
      align-items: center;
      gap: 0.5rem;
      margin-bottom: 0.75rem;
    }

    .card-dot {
      width: 8px;
      height: 8px;
      border-radius: 50%;
      display: inline-block;
    }

    .card-dot.accent { background: var(--color-accent); }
    .card-dot.link { background: var(--color-link); }
    .card-dot.muted { background: var(--color-muted); }

    .card h3 {
      font-size: 1rem;
      font-weight: 600;
      color: var(--color-ink);
    }

    .card p {
      font-size: 0.86rem;
      color: var(--color-muted);
      line-height: 1.45;
    }

    .footer {
      border-top: 1px solid var(--color-rule);
      padding-top: 1.25rem;
      font-family: var(--font-mono);
      font-size: 0.72rem;
      color: var(--color-soft);
      display: flex;
      justify-content: space-between;
      width: 100%;
    }
  </style>
</head>
<body>
  <div class="frame">
    <p class="eyebrow">SIH26123 · Bharat Electronics Limited (BEL)</p>
    <h1>EdgeBot High-Level System Architecture</h1>
    <p class="subtitle">Decentralized peer-to-peer fleet coordination for Autonomous Mobile Robots operating without central dispatch servers or continuous cloud connectivity.</p>

    <div class="diagram-container">
      <svg viewBox="0 0 1080 500" xmlns="http://www.w3.org/2000/svg">
        <defs>
          <pattern id="dots" width="24" height="24" patternUnits="userSpaceOnUse">
            <circle cx="2" cy="2" r="0.9" fill="rgba(45,49,66,0.08)"/>
          </pattern>
          <marker id="arrow" markerWidth="8" markerHeight="6" refX="7" refY="3" orient="auto">
            <polygon points="0 0, 8 3, 0 6" fill="#4f5d75"/>
          </marker>
          <marker id="arrow-accent" markerWidth="8" markerHeight="6" refX="7" refY="3" orient="auto">
            <polygon points="0 0, 8 3, 0 6" fill="#eb6c36"/>
          </marker>
          <marker id="arrow-link" markerWidth="8" markerHeight="6" refX="7" refY="3" orient="auto">
            <polygon points="0 0, 8 3, 0 6" fill="#2e5aa8"/>
          </marker>
          <marker id="arrow-soft" markerWidth="8" markerHeight="6" refX="7" refY="3" orient="auto">
            <polygon points="0 0, 8 3, 0 6" fill="#7a8399"/>
          </marker>
        </defs>

        <!-- Background -->
        <rect width="100%" height="100%" fill="#f5f5f5"/>
        <rect width="100%" height="100%" fill="url(#dots)" opacity="0.6"/>

        <!-- ==================== ZONES (Drawn before arrows & nodes) ==================== -->
        <!-- Zone 1: AMR Edge Compute -->
        <rect x="40" y="44" width="344" height="392" rx="8"
              fill="rgba(45,49,66,0.02)" stroke="rgba(45,49,66,0.12)" stroke-width="0.8"/>
        <rect x="76" y="48" width="168" height="14" rx="2" fill="#f5f5f5"/>
        <text x="160" y="58" fill="rgba(45,49,66,0.50)" font-size="7.5" font-family="'Geist Mono', monospace"
              text-anchor="middle" letter-spacing="0.14em">AMR EDGE COMPUTE (NODE #1)</text>

        <!-- Zone 2: P2P Transport Fabric -->
        <rect x="424" y="148" width="272" height="288" rx="8"
              fill="rgba(45,49,66,0.02)" stroke="rgba(45,49,66,0.12)" stroke-width="0.8"/>
        <rect x="460" y="152" width="164" height="14" rx="2" fill="#f5f5f5"/>
        <text x="542" y="162" fill="rgba(45,49,66,0.50)" font-size="7.5" font-family="'Geist Mono', monospace"
              text-anchor="middle" letter-spacing="0.14em">P2P TRANSPORT FABRIC</text>

        <!-- Zone 3: Monitoring & Harness -->
        <rect x="736" y="44" width="304" height="392" rx="8"
              fill="rgba(45,49,66,0.02)" stroke="rgba(45,49,66,0.12)" stroke-width="0.8"/>
        <rect x="780" y="48" width="180" height="14" rx="2" fill="#f5f5f5"/>
        <text x="870" y="58" fill="rgba(45,49,66,0.50)" font-size="7.5" font-family="'Geist Mono', monospace"
              text-anchor="middle" letter-spacing="0.14em">SUPERVISION &amp; HARNESS</text>

        <!-- ==================== CONNECTORS (Drawn before nodes) ==================== -->
        <!-- 1. Perception -> EdgeBot Core (Vertical) -->
        <line x1="212" y1="136" x2="212" y2="188" stroke="#4f5d75" stroke-width="1.2" marker-end="url(#arrow)"/>

        <!-- 2. EdgeBot Core -> Motor HAL (Vertical) -->
        <line x1="212" y1="272" x2="212" y2="344" stroke="#4f5d75" stroke-width="1.2" marker-end="url(#arrow)"/>

        <!-- 3. EdgeBot Core -> P2P Mesh (Horizontal, Accent) -->
        <line x1="324" y1="230" x2="456" y2="230" stroke="#eb6c36" stroke-width="1.4" marker-end="url(#arrow-accent)"/>

        <!-- 4. P2P Mesh -> Peer AMRs (Vertical) -->
        <line x1="560" y1="264" x2="560" y2="344" stroke="#4f5d75" stroke-width="1.2" marker-end="url(#arrow)"/>

        <!-- 5. Scenario Injector -> P2P Mesh (Orthogonal Elbow with r=8, Dashed) -->
        <path d="M 888,136 V 176 Q 888,184 880,184 H 616 Q 608,184 608,192 V 200"
              fill="none" stroke="#7a8399" stroke-width="1" stroke-dasharray="4,3" marker-end="url(#arrow-soft)"/>

        <!-- 6. P2P Mesh -> Telemetry Console (Horizontal) -->
        <line x1="664" y1="230" x2="788" y2="230" stroke="#2e5aa8" stroke-width="1.2" marker-end="url(#arrow-link)"/>

        <!-- ==================== CONNECTOR LABELS (With Opaque Masks & 6-10px gaps) ==================== -->
        <!-- Label 1: SENSE -->
        <rect x="190" y="152" width="44" height="12" rx="2" fill="#f5f5f5"/>
        <text x="212" y="161" fill="#4f5d75" font-size="7.5" font-family="'Geist Mono', monospace" text-anchor="middle" letter-spacing="0.08em">SENSE</text>

        <!-- Label 2: ACTUATE -->
        <rect x="184" y="300" width="56" height="12" rx="2" fill="#f5f5f5"/>
        <text x="212" y="309" fill="#4f5d75" font-size="7.5" font-family="'Geist Mono', monospace" text-anchor="middle" letter-spacing="0.08em">ACTUATE</text>

        <!-- Label 3: GOSSIP -->
        <rect x="362" y="210" width="54" height="12" rx="2" fill="#f5f5f5"/>
        <text x="389" y="219" fill="#eb6c36" font-size="7.5" font-family="'Geist Mono', monospace" text-anchor="middle" letter-spacing="0.08em">GOSSIP</text>

        <!-- Label 4: CONSENSUS -->
        <rect x="526" y="296" width="68" height="12" rx="2" fill="#f5f5f5"/>
        <text x="560" y="305" fill="#4f5d75" font-size="7.5" font-family="'Geist Mono', monospace" text-anchor="middle" letter-spacing="0.08em">CONSENSUS</text>

        <!-- Label 5: FAULTS -->
        <rect x="716" y="174" width="50" height="12" rx="2" fill="#f5f5f5"/>
        <text x="741" y="183" fill="#7a8399" font-size="7.5" font-family="'Geist Mono', monospace" text-anchor="middle" letter-spacing="0.08em">FAULTS</text>

        <!-- Label 6: TELEMETRY -->
        <rect x="696" y="210" width="64" height="12" rx="2" fill="#f5f5f5"/>
        <text x="728" y="219" fill="#2e5aa8" font-size="7.5" font-family="'Geist Mono', monospace" text-anchor="middle" letter-spacing="0.08em">TELEMETRY</text>

        <!-- ==================== NODES (Full Pattern) ==================== -->
        <!-- Node 1: Perception (Input) -->
        <g>
          <rect x="116" y="80" width="192" height="56" rx="6" fill="#f5f5f5"/>
          <rect x="116" y="80" width="192" height="56" rx="6" fill="rgba(79,93,117,0.08)" stroke="#7a8399" stroke-width="1"/>
          <rect x="124" y="86" width="34" height="12" rx="2" fill="transparent" stroke="rgba(122,131,153,0.4)" stroke-width="0.8"/>
          <text x="141" y="95" fill="#7a8399" font-size="7" font-family="'Geist Mono', monospace" text-anchor="middle" letter-spacing="0.08em">INPUT</text>
          <text x="212" y="106" fill="#2d3142" font-size="12" font-weight="600" font-family="'Geist', sans-serif" text-anchor="middle">Perception &amp; Odometry</text>
          <text x="212" y="122" fill="#4f5d75" font-size="9" font-family="'Geist Mono', monospace" text-anchor="middle">LiDAR · Ultrasonic · 30 Hz</text>
        </g>

        <!-- Node 2: EdgeBot Core Engine (Focal Accent) -->
        <g>
          <rect x="100" y="188" width="224" height="84" rx="6" fill="#f5f5f5"/>
          <rect x="100" y="188" width="224" height="84" rx="6" fill="rgba(235,108,54,0.08)" stroke="#eb6c36" stroke-width="1.4"/>
          <rect x="108" y="194" width="32" height="12" rx="2" fill="transparent" stroke="rgba(235,108,54,0.5)" stroke-width="0.8"/>
          <text x="124" y="203" fill="#eb6c36" font-size="7" font-weight="600" font-family="'Geist Mono', monospace" text-anchor="middle" letter-spacing="0.08em">CORE</text>
          <text x="212" y="222" fill="#2d3142" font-size="13" font-weight="600" font-family="'Geist', sans-serif" text-anchor="middle">EdgeBot Coordination Engine</text>
          <text x="212" y="240" fill="#4f5d75" font-size="9" font-family="'Geist Mono', monospace" text-anchor="middle">Space-Time A* · CNP Auctions</text>
          <text x="212" y="255" fill="#eb6c36" font-size="8.5" font-weight="500" font-family="'Geist Mono', monospace" text-anchor="middle">Wait-For Graph DFS (Rust 2024)</text>
        </g>

        <!-- Node 3: Motor Controller (Backend / Actuator) -->
        <g>
          <rect x="116" y="344" width="192" height="60" rx="6" fill="#f5f5f5"/>
          <rect x="116" y="344" width="192" height="60" rx="6" fill="#ffffff" stroke="#2d3142" stroke-width="1"/>
          <rect x="124" y="350" width="48" height="12" rx="2" fill="transparent" stroke="rgba(45,49,66,0.4)" stroke-width="0.8"/>
          <text x="148" y="359" fill="#2d3142" font-size="7" font-family="'Geist Mono', monospace" text-anchor="middle" letter-spacing="0.08em">ACTUATOR</text>
          <text x="212" y="372" fill="#2d3142" font-size="12" font-weight="600" font-family="'Geist', sans-serif" text-anchor="middle">Motor HAL &amp; Safety Stop</text>
          <text x="212" y="388" fill="#4f5d75" font-size="9" font-family="'Geist Mono', monospace" text-anchor="middle">Arduino Uno · L298N · ISO 3691-4</text>
        </g>

        <!-- Node 4: P2P Mesh Fabric (Transport / Store) -->
        <g>
          <rect x="456" y="200" width="208" height="64" rx="6" fill="#f5f5f5"/>
          <rect x="456" y="200" width="208" height="64" rx="6" fill="rgba(45,49,66,0.05)" stroke="#4f5d75" stroke-width="1.2"/>
          <rect x="464" y="206" width="34" height="12" rx="2" fill="transparent" stroke="rgba(79,93,117,0.4)" stroke-width="0.8"/>
          <text x="481" y="215" fill="#4f5d75" font-size="7" font-family="'Geist Mono', monospace" text-anchor="middle" letter-spacing="0.08em">FABRIC</text>
          <text x="560" y="228" fill="#2d3142" font-size="12" font-weight="600" font-family="'Geist', sans-serif" text-anchor="middle">Zero-Broker P2P Mesh</text>
          <text x="560" y="246" fill="#4f5d75" font-size="9" font-family="'Geist Mono', monospace" text-anchor="middle">UDP Multicast · 239.0.26.123:26123</text>
        </g>

        <!-- Node 5: Peer Robots (Backend / Distributed peers) -->
        <g>
          <rect x="456" y="344" width="208" height="60" rx="6" fill="#f5f5f5"/>
          <rect x="456" y="344" width="208" height="60" rx="6" fill="#ffffff" stroke="#2d3142" stroke-width="1"/>
          <rect x="464" y="350" width="36" height="12" rx="2" fill="transparent" stroke="rgba(45,49,66,0.4)" stroke-width="0.8"/>
          <text x="482" y="359" fill="#2d3142" font-size="7" font-family="'Geist Mono', monospace" text-anchor="middle" letter-spacing="0.08em">PEERS</text>
          <text x="560" y="372" fill="#2d3142" font-size="12" font-weight="600" font-family="'Geist', sans-serif" text-anchor="middle">Fleet Peers (#2 ... #N)</text>
          <text x="560" y="388" fill="#4f5d75" font-size="9" font-family="'Geist Mono', monospace" text-anchor="middle">Autonomous Symmetric Nodes</text>
        </g>

        <!-- Node 6: Scenario Injector (Optional / Async) -->
        <g>
          <rect x="788" y="80" width="200" height="56" rx="6" fill="#f5f5f5"/>
          <rect x="788" y="80" width="200" height="56" rx="6" fill="rgba(45,49,66,0.02)" stroke="rgba(45,49,66,0.30)" stroke-width="1" stroke-dasharray="4,3"/>
          <rect x="796" y="86" width="44" height="12" rx="2" fill="transparent" stroke="rgba(45,49,66,0.3)" stroke-width="0.8"/>
          <text x="818" y="95" fill="#4f5d75" font-size="7" font-family="'Geist Mono', monospace" text-anchor="middle" letter-spacing="0.08em">HARNESS</text>
          <text x="888" y="106" fill="#2d3142" font-size="12" font-weight="600" font-family="'Geist', sans-serif" text-anchor="middle">Scenario &amp; Fault Injector</text>
          <text x="888" y="122" fill="#4f5d75" font-size="9" font-family="'Geist Mono', monospace" text-anchor="middle">Choke-Points · Obstacles · Kills</text>
        </g>

        <!-- Node 7: Telemetry Console (External / Read-only) -->
        <g>
          <rect x="788" y="200" width="200" height="64" rx="6" fill="#f5f5f5"/>
          <rect x="788" y="200" width="200" height="64" rx="6" fill="rgba(46,90,168,0.04)" stroke="#2e5aa8" stroke-width="1"/>
          <rect x="796" y="206" width="56" height="12" rx="2" fill="transparent" stroke="rgba(46,90,168,0.4)" stroke-width="0.8"/>
          <text x="824" y="215" fill="#2e5aa8" font-size="7" font-family="'Geist Mono', monospace" text-anchor="middle" letter-spacing="0.08em">TELEMETRY</text>
          <text x="888" y="228" fill="#2d3142" font-size="12" font-weight="600" font-family="'Geist', sans-serif" text-anchor="middle">Passive Fleet Console</text>
          <text x="888" y="246" fill="#2e5aa8" font-size="9" font-family="'Geist Mono', monospace" text-anchor="middle">Axum WebSockets · HTML5 Canvas</text>
        </g>

        <!-- ==================== LEGEND STRIP ==================== -->
        <line x1="40" y1="468" x2="1040" y2="468" stroke="rgba(45,49,66,0.12)" stroke-width="0.8"/>
        <text x="40" y="488" fill="#4f5d75" font-size="8" font-family="'Geist Mono', monospace" letter-spacing="0.14em">LEGEND</text>

        <!-- Legend Item 1 -->
        <rect x="120" y="480" width="16" height="10" rx="2" fill="rgba(235,108,54,0.08)" stroke="#eb6c36" stroke-width="1.2"/>
        <text x="144" y="488" fill="#2d3142" font-size="9" font-family="'Geist', sans-serif">Focal Coordination Engine</text>

        <!-- Legend Item 2 -->
        <rect x="340" y="480" width="16" height="10" rx="2" fill="#ffffff" stroke="#2d3142" stroke-width="1"/>
        <text x="364" y="488" fill="#2d3142" font-size="9" font-family="'Geist', sans-serif">Execution &amp; Peer Nodes</text>

        <!-- Legend Item 3 -->
        <rect x="540" y="480" width="16" height="10" rx="2" fill="rgba(45,49,66,0.05)" stroke="#4f5d75" stroke-width="1"/>
        <text x="564" y="488" fill="#2d3142" font-size="9" font-family="'Geist', sans-serif">P2P Transport Fabric</text>

        <!-- Legend Item 4 -->
        <line x1="720" y1="485" x2="744" y2="485" stroke="#7a8399" stroke-width="1" stroke-dasharray="3,3" marker-end="url(#arrow-soft)"/>
        <text x="752" y="488" fill="#2d3142" font-size="9" font-family="'Geist', sans-serif">Asynchronous / Fault Injection</text>

        <!-- Legend Item 5 -->
        <line x1="940" y1="485" x2="964" y2="485" stroke="#2e5aa8" stroke-width="1.2" marker-end="url(#arrow-link)"/>
        <text x="972" y="488" fill="#2d3142" font-size="9" font-family="'Geist', sans-serif">Passive Telemetry</text>
      </svg>
    </div>

    <!-- Editorial Summary Cards -->
    <div class="card-grid">
      <div class="card">
        <p class="card-eyebrow">DISTRIBUTED CORE</p>
        <div class="card-header">
          <span class="card-dot accent"></span>
          <h3>Zero-SPOF Edge Engine</h3>
        </div>
        <p>Each robot runs an independent pure Rust 2024 actor loop (Sense → Decide → Flush → Move → Evaluate). Path planning uses 3D Space-Time A* with local reservation tables; deadlocks are detected and resolved locally using Wait-For-Graph DFS cycles.</p>
      </div>

      <div class="card">
        <p class="card-eyebrow">TRANSPORT FABRIC</p>
        <div class="card-header">
          <span class="card-dot muted"></span>
          <h3>Autonomous P2P Mesh</h3>
        </div>
        <p>AMRs communicate peer-to-peer over UDP Multicast (239.0.26.123:26123) with ZeroMQ PUB/SUB sockets. Soft-state heartbeats (2 Hz) and 10 Hz pose broadcasts enable immediate dead-chassis discovery without a central broker.</p>
      </div>

      <div class="card">
        <p class="card-eyebrow">SAFETY &amp; INTEGRATION</p>
        <div class="card-header">
          <span class="card-dot link"></span>
          <h3>Hardware &amp; ISO 3691-4</h3>
        </div>
        <p>Hardware-in-the-Loop Arduino Uno HAL operates an ultrasonic emergency stop (&lt;50ms) and 500ms serial watchdog. Passive Axum WebSocket dashboard renders real-time state with zero arbitration authority.</p>
      </div>
    </div>

    <div class="footer">
      <span>EdgeBot Architecture · Smart India Hackathon 2026 (SIH26123)</span>
      <span>Team Cold Start · Bharat Electronics Limited (BEL)</span>
    </div>
  </div>
</body>
</html>
"""

def extract_svg(html_content):
    start_tag = "<svg"
    end_tag = "</svg>"
    start_idx = html_content.find(start_tag)
    end_idx = html_content.find(end_tag) + len(end_tag)
    svg_body = html_content[start_idx:end_idx]
    
    # Wrap in standalone SVG with fonts and styles
    standalone = f'''<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1080 500" width="1080" height="500" style="background:#f5f5f5; font-family: system-ui, -apple-system, sans-serif;">
  <style>
    @import url('https://fonts.googleapis.com/css2?family=Geist:wght@400;500;600&amp;family=Geist+Mono:wght@400;500;600&amp;display=swap');
  </style>
{svg_body[svg_body.find('>')+1:svg_body.rfind('<')]}
</svg>'''
    return standalone

def main():
    out_dir = "docs/diagrams"
    os.makedirs(out_dir, exist_ok=True)

    # 1. HTML file
    html_path = os.path.join(out_dir, "high_level_architecture.html")
    with open(html_path, "w", encoding="utf-8") as f:
        f.write(HTML_CONTENT)
    print(f"Generated HTML: {html_path}")

    # 2. Standalone SVG file
    svg_path = os.path.join(out_dir, "high_level_architecture.svg")
    svg_content = extract_svg(HTML_CONTENT)
    with open(svg_path, "w", encoding="utf-8") as f:
        f.write(svg_content)
    print(f"Generated SVG: {svg_path}")

    # 3. High-res PNG of full page layout
    png_full = os.path.join(out_dir, "high_level_architecture.png")
    cmd_full = [
        "google-chrome",
        "--headless",
        "--disable-gpu",
        f"--screenshot={png_full}",
        "--window-size=1240,960",
        f"file://{os.path.abspath(html_path)}"
    ]
    subprocess.run(cmd_full, check=True)
    print(f"Generated Full Page PNG: {png_full}")

    # 4. Standalone diagram-only PNG
    png_diag = os.path.join(out_dir, "high_level_architecture_diagram.png")
    # Render SVG via a tiny HTML wrapper to ensure perfect font and viewBox rendering
    wrapper_html = f"""<!DOCTYPE html>
<html><body style="margin:0;padding:0;background:#f5f5f5;">{svg_content}</body></html>"""
    wrapper_path = "/tmp/diag_wrapper.html"
    with open(wrapper_path, "w") as f:
        f.write(wrapper_html)
    cmd_diag = [
        "google-chrome",
        "--headless",
        "--disable-gpu",
        f"--screenshot={png_diag}",
        "--window-size=1080,500",
        f"file://{wrapper_path}"
    ]
    subprocess.run(cmd_diag, check=True)
    print(f"Generated Standalone Diagram PNG: {png_diag}")

if __name__ == "__main__":
    main()
