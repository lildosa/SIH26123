#!/usr/bin/env python3
"""
Generates the EdgeBot Distributed Architecture diagram as:
1. docs/diagrams/architecture.excalidraw (Excalidraw v2 format)
2. docs/diagrams/architecture.svg (High-res standalone vector SVG)
"""

import json
import uuid
import os

def make_id():
    return uuid.uuid4().hex[:16]

def create_rect(x, y, w, h, bg_color="#ffffff", stroke_color="#1e1e1e", stroke_style="solid", stroke_width=2, roundness=3, fill_style="solid"):
    return {
        "id": make_id(),
        "type": "rectangle",
        "x": x,
        "y": y,
        "width": w,
        "height": h,
        "angle": 0,
        "strokeColor": stroke_color,
        "backgroundColor": bg_color,
        "fillStyle": fill_style,
        "strokeWidth": stroke_width,
        "strokeStyle": stroke_style,
        "roughness": 1,
        "opacity": 100,
        "groupIds": [],
        "frameId": None,
        "roundness": {"type": roundness} if roundness else None,
        "seed": int(uuid.uuid4().int % 1000000),
        "version": 1,
        "versionNonce": 1,
        "isDeleted": False,
        "boundElements": [],
        "updated": 1,
        "link": None,
        "locked": False
    }

def create_text(x, y, text, font_size=16, font_family=1, text_align="center", stroke_color="#1e1e1e"):
    lines = text.split("\n")
    line_height = font_size * 1.25
    approx_w = max(len(l) for l in lines) * (font_size * 0.6)
    approx_h = len(lines) * line_height
    return {
        "id": make_id(),
        "type": "text",
        "x": x,
        "y": y,
        "width": approx_w,
        "height": approx_h,
        "angle": 0,
        "strokeColor": stroke_color,
        "backgroundColor": "transparent",
        "fillStyle": "solid",
        "strokeWidth": 1,
        "strokeStyle": "solid",
        "roughness": 0,
        "opacity": 100,
        "groupIds": [],
        "frameId": None,
        "roundness": None,
        "seed": int(uuid.uuid4().int % 1000000),
        "version": 1,
        "versionNonce": 1,
        "isDeleted": False,
        "boundElements": None,
        "updated": 1,
        "link": None,
        "locked": False,
        "text": text,
        "fontSize": font_size,
        "fontFamily": font_family,
        "textAlign": text_align,
        "verticalAlign": "top",
        "baseline": font_size,
        "containerId": None,
        "originalText": text,
        "lineHeight": 1.25
    }

def create_arrow(start_x, start_y, end_x, end_y, stroke_color="#1e1e1e", stroke_style="solid", stroke_width=2, end_arrow=True):
    dx = end_x - start_x
    dy = end_y - start_y
    return {
        "id": make_id(),
        "type": "arrow",
        "x": start_x,
        "y": start_y,
        "width": abs(dx),
        "height": abs(dy),
        "angle": 0,
        "strokeColor": stroke_color,
        "backgroundColor": "transparent",
        "fillStyle": "solid",
        "strokeWidth": stroke_width,
        "strokeStyle": stroke_style,
        "roughness": 1,
        "opacity": 100,
        "groupIds": [],
        "frameId": None,
        "roundness": {"type": 2},
        "seed": int(uuid.uuid4().int % 1000000),
        "version": 1,
        "versionNonce": 1,
        "isDeleted": False,
        "boundElements": None,
        "updated": 1,
        "link": None,
        "locked": False,
        "points": [[0, 0], [dx, dy]],
        "lastCommittedPoint": None,
        "startBinding": None,
        "endBinding": None,
        "startArrowhead": None,
        "endArrowhead": "arrow" if end_arrow else None
    }

def build_excalidraw():
    elements = []

    # Title
    elements.append(create_text(350, 40, "SIH26123 — Decentralized AMR Fleet Architecture", font_size=26, font_family=1, text_align="center", stroke_color="#0f172a"))

    # -------------------------------------------------------------
    # AMR #1 (Left Zone)
    # -------------------------------------------------------------
    elements.append(create_rect(50, 110, 360, 460, bg_color="#e3f2fd", stroke_color="#1976d2", stroke_width=2, roundness=3))
    elements.append(create_text(70, 125, "AMR #1 — Edge Node (Raspberry Pi / Jetson)", font_size=15, font_family=1, text_align="left", stroke_color="#0d47a1"))

    # AMR #1 components
    elements.append(create_rect(75, 160, 140, 50, bg_color="#90caf9", stroke_color="#0d47a1", roundness=3))
    elements.append(create_text(85, 170, "Task Bidder\n(contract-net)", font_size=13, font_family=1, text_align="center"))

    elements.append(create_rect(245, 160, 140, 50, bg_color="#90caf9", stroke_color="#0d47a1", roundness=3))
    elements.append(create_text(255, 170, "Localization\n(grid pose)", font_size=13, font_family=1, text_align="center"))

    elements.append(create_rect(145, 260, 180, 50, bg_color="#90caf9", stroke_color="#0d47a1", roundness=3))
    elements.append(create_text(155, 270, "Planner\nSpace-Time A* / D* Lite", font_size=13, font_family=1, text_align="center"))

    elements.append(create_rect(125, 360, 220, 55, bg_color="#64b5f6", stroke_color="#0d47a1", roundness=3))
    elements.append(create_text(135, 370, "Traffic Negotiator\nreservations + wait-for graph", font_size=13, font_family=1, text_align="center"))

    elements.append(create_rect(160, 470, 150, 45, bg_color="#42a5f5", stroke_color="#0d47a1", roundness=3))
    elements.append(create_text(175, 482, "Motion Controller", font_size=14, font_family=1, text_align="center"))

    # AMR #1 internal arrows
    elements.append(create_arrow(145, 210, 210, 260))
    elements.append(create_arrow(315, 210, 260, 260))
    elements.append(create_arrow(235, 310, 235, 360))
    elements.append(create_arrow(235, 415, 235, 470))

    # -------------------------------------------------------------
    # AMR #2 (Right Zone)
    # -------------------------------------------------------------
    elements.append(create_rect(830, 110, 360, 460, bg_color="#e3f2fd", stroke_color="#1976d2", stroke_width=2, roundness=3))
    elements.append(create_text(850, 125, "AMR #2 — Edge Node", font_size=15, font_family=1, text_align="left", stroke_color="#0d47a1"))

    elements.append(create_rect(855, 160, 140, 50, bg_color="#90caf9", stroke_color="#0d47a1", roundness=3))
    elements.append(create_text(875, 175, "Task Bidder", font_size=13, font_family=1, text_align="center"))

    elements.append(create_rect(1025, 160, 140, 50, bg_color="#90caf9", stroke_color="#0d47a1", roundness=3))
    elements.append(create_text(1045, 175, "Localization", font_size=13, font_family=1, text_align="center"))

    elements.append(create_rect(925, 260, 180, 50, bg_color="#90caf9", stroke_color="#0d47a1", roundness=3))
    elements.append(create_text(945, 270, "Planner\nA* / D* Lite", font_size=13, font_family=1, text_align="center"))

    elements.append(create_rect(915, 360, 200, 55, bg_color="#64b5f6", stroke_color="#0d47a1", roundness=3))
    elements.append(create_text(940, 377, "Traffic Negotiator", font_size=14, font_family=1, text_align="center"))

    elements.append(create_rect(940, 470, 150, 45, bg_color="#42a5f5", stroke_color="#0d47a1", roundness=3))
    elements.append(create_text(970, 482, "Controller", font_size=14, font_family=1, text_align="center"))

    # AMR #2 internal arrows
    elements.append(create_arrow(925, 210, 990, 260))
    elements.append(create_arrow(1095, 210, 1040, 260))
    elements.append(create_arrow(1015, 310, 1015, 360))
    elements.append(create_arrow(1015, 415, 1015, 470))

    # -------------------------------------------------------------
    # Middle Nodes: AMR #3...#N & Scenario Injector
    # -------------------------------------------------------------
    elements.append(create_rect(440, 320, 180, 85, bg_color="#e3f2fd", stroke_color="#1976d2", roundness=3))
    elements.append(create_text(450, 330, "AMR #3 ... #N — Edge Nodes\n\nsame stack per robot\n(self-contained process)", font_size=12, font_family=1, text_align="center", stroke_color="#0d47a1"))

    elements.append(create_rect(640, 320, 170, 85, bg_color="#f3e5f5", stroke_color="#7b1fa2", roundness=3))
    elements.append(create_text(650, 340, "Scenario Injector\n\nblocked aisle · robot kill\ntask bursts", font_size=12, font_family=1, text_align="center", stroke_color="#4a148c"))

    # -------------------------------------------------------------
    # P2P MESH Layer (Center Pill)
    # -------------------------------------------------------------
    elements.append(create_rect(360, 480, 520, 65, bg_color="#ffe0b2", stroke_color="#e65100", stroke_width=2, roundness=3))
    elements.append(create_text(375, 490, "P2P MESH (ZeroMQ PUB/SUB + UDP discovery)\npose 10 Hz · heartbeat 2 Hz · intent · reservations · auction/bids", font_size=13, font_family=1, text_align="center", stroke_color="#bf360c"))

    # -------------------------------------------------------------
    # Fleet Dashboard (Bottom)
    # -------------------------------------------------------------
    elements.append(create_rect(410, 610, 420, 75, bg_color="#c8e6c9", stroke_color="#2e7d32", stroke_width=2, roundness=3))
    elements.append(create_text(420, 620, "Fleet Dashboard\npositions · battery · reserved windows · live metrics\nREAD-ONLY observer — never coordinates", font_size=13, font_family=1, text_align="center", stroke_color="#1b5e20"))

    # -------------------------------------------------------------
    # Cross-layer Arrows & Labels
    # -------------------------------------------------------------
    # AMR #1 Negotiator to P2P Mesh
    elements.append(create_arrow(280, 415, 460, 480, stroke_color="#0d47a1", stroke_width=2))
    elements.append(create_text(320, 435, "publish / subscribe", font_size=11, font_family=1, stroke_color="#475569"))

    # AMR #3...#N to P2P Mesh
    elements.append(create_arrow(530, 405, 530, 480, stroke_color="#0d47a1", stroke_width=2))

    # Scenario Injector to P2P Mesh (Dotted)
    elements.append(create_arrow(720, 405, 680, 480, stroke_color="#7b1fa2", stroke_style="dotted", stroke_width=2))
    elements.append(create_text(710, 435, "injects", font_size=11, font_family=1, stroke_color="#7b1fa2"))

    # AMR #2 Negotiator to P2P Mesh
    elements.append(create_arrow(960, 415, 780, 480, stroke_color="#0d47a1", stroke_width=2))

    # P2P Mesh to Fleet Dashboard (Dashed)
    elements.append(create_arrow(620, 545, 620, 610, stroke_color="#2e7d32", stroke_style="dashed", stroke_width=2))
    elements.append(create_text(630, 568, "subscribe", font_size=11, font_family=1, stroke_color="#2e7d32"))

    excalidraw_doc = {
        "type": "excalidraw",
        "version": 2,
        "source": "https://excalidraw.com",
        "elements": elements,
        "appState": {
            "viewBackgroundColor": "#ffffff",
            "gridSize": None
        },
        "files": {}
    }

    return excalidraw_doc

def build_svg():
    svg = '''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1240 730" width="1240" height="730" style="background:#ffffff; font-family: 'Segoe UI', system-ui, -apple-system, Roboto, Helvetica, Arial, sans-serif;">
  <defs>
    <filter id="shadow" x="-5%" y="-5%" width="110%" height="110%">
      <feDropShadow dx="2" dy="3" stdDeviation="3" flood-opacity="0.08"/>
    </filter>
    <marker id="arrowhead" markerWidth="10" markerHeight="7" refX="9" refY="3.5" orient="auto">
      <polygon points="0 0, 10 3.5, 0 7" fill="#1e293b"/>
    </marker>
    <marker id="arrowhead-blue" markerWidth="10" markerHeight="7" refX="9" refY="3.5" orient="auto">
      <polygon points="0 0, 10 3.5, 0 7" fill="#0284c7"/>
    </marker>
    <marker id="arrowhead-purple" markerWidth="10" markerHeight="7" refX="9" refY="3.5" orient="auto">
      <polygon points="0 0, 10 3.5, 0 7" fill="#9333ea"/>
    </marker>
    <marker id="arrowhead-green" markerWidth="10" markerHeight="7" refX="9" refY="3.5" orient="auto">
      <polygon points="0 0, 10 3.5, 0 7" fill="#16a34a"/>
    </marker>
  </defs>

  <!-- Title -->
  <text x="620" y="55" font-size="26" font-weight="bold" fill="#0f172a" text-anchor="middle">SIH26123 — Decentralized AMR Fleet Architecture</text>

  <!-- AMR #1 Container -->
  <rect x="50" y="90" width="370" height="470" rx="16" fill="#f0f9ff" stroke="#0284c7" stroke-width="2" filter="url(#shadow)"/>
  <text x="75" y="125" font-size="16" font-weight="bold" fill="#0369a1">AMR #1 — Edge Node (Raspberry Pi / Jetson)</text>

  <!-- AMR #1 Inner Nodes -->
  <rect x="75" y="150" width="145" height="52" rx="10" fill="#bae6fd" stroke="#0284c7" stroke-width="1.5"/>
  <text x="147" y="172" font-size="13" font-weight="600" fill="#0c4a6e" text-anchor="middle">Task Bidder</text>
  <text x="147" y="190" font-size="11" fill="#0369a1" text-anchor="middle">(contract-net)</text>

  <rect x="250" y="150" width="145" height="52" rx="10" fill="#bae6fd" stroke="#0284c7" stroke-width="1.5"/>
  <text x="322" y="172" font-size="13" font-weight="600" fill="#0c4a6e" text-anchor="middle">Localization</text>
  <text x="322" y="190" font-size="11" fill="#0369a1" text-anchor="middle">(grid pose)</text>

  <rect x="145" y="245" width="180" height="54" rx="10" fill="#bae6fd" stroke="#0284c7" stroke-width="1.5"/>
  <text x="235" y="267" font-size="13" font-weight="600" fill="#0c4a6e" text-anchor="middle">Planner</text>
  <text x="235" y="286" font-size="11" fill="#0369a1" text-anchor="middle">Space-Time A* / D* Lite</text>

  <rect x="125" y="340" width="220" height="56" rx="10" fill="#7dd3fc" stroke="#0284c7" stroke-width="1.5"/>
  <text x="235" y="362" font-size="14" font-weight="600" fill="#0c4a6e" text-anchor="middle">Traffic Negotiator</text>
  <text x="235" y="382" font-size="11" fill="#0369a1" text-anchor="middle">reservations + wait-for graph</text>

  <rect x="160" y="445" width="150" height="48" rx="10" fill="#38bdf8" stroke="#0284c7" stroke-width="1.5"/>
  <text x="235" y="475" font-size="14" font-weight="700" fill="#0c4a6e" text-anchor="middle">Motion Controller</text>

  <!-- AMR #1 Internal Connectors -->
  <line x1="147" y1="202" x2="200" y2="245" stroke="#0f172a" stroke-width="1.5" marker-end="url(#arrowhead)"/>
  <line x1="322" y1="202" x2="270" y2="245" stroke="#0f172a" stroke-width="1.5" marker-end="url(#arrowhead)"/>
  <line x1="235" y1="299" x2="235" y2="340" stroke="#0f172a" stroke-width="1.5" marker-end="url(#arrowhead)"/>
  <line x1="235" y1="396" x2="235" y2="445" stroke="#0f172a" stroke-width="1.5" marker-end="url(#arrowhead)"/>

  <!-- AMR #2 Container -->
  <rect x="820" y="90" width="370" height="470" rx="16" fill="#f0f9ff" stroke="#0284c7" stroke-width="2" filter="url(#shadow)"/>
  <text x="845" y="125" font-size="16" font-weight="bold" fill="#0369a1">AMR #2 — Edge Node</text>

  <!-- AMR #2 Inner Nodes -->
  <rect x="845" y="150" width="145" height="52" rx="10" fill="#bae6fd" stroke="#0284c7" stroke-width="1.5"/>
  <text x="917" y="181" font-size="13" font-weight="600" fill="#0c4a6e" text-anchor="middle">Task Bidder</text>

  <rect x="1020" y="150" width="145" height="52" rx="10" fill="#bae6fd" stroke="#0284c7" stroke-width="1.5"/>
  <text x="1092" y="181" font-size="13" font-weight="600" fill="#0c4a6e" text-anchor="middle">Localization</text>

  <rect x="915" y="245" width="180" height="54" rx="10" fill="#bae6fd" stroke="#0284c7" stroke-width="1.5"/>
  <text x="1005" y="267" font-size="13" font-weight="600" fill="#0c4a6e" text-anchor="middle">Planner</text>
  <text x="1005" y="286" font-size="11" fill="#0369a1" text-anchor="middle">A* / D* Lite</text>

  <rect x="905" y="340" width="200" height="56" rx="10" fill="#7dd3fc" stroke="#0284c7" stroke-width="1.5"/>
  <text x="1005" y="373" font-size="14" font-weight="600" fill="#0c4a6e" text-anchor="middle">Traffic Negotiator</text>

  <rect x="930" y="445" width="150" height="48" rx="10" fill="#38bdf8" stroke="#0284c7" stroke-width="1.5"/>
  <text x="1005" y="475" font-size="14" font-weight="700" fill="#0c4a6e" text-anchor="middle">Controller</text>

  <!-- AMR #2 Internal Connectors -->
  <line x1="917" y1="202" x2="970" y2="245" stroke="#0f172a" stroke-width="1.5" marker-end="url(#arrowhead)"/>
  <line x1="1092" y1="202" x2="1040" y2="245" stroke="#0f172a" stroke-width="1.5" marker-end="url(#arrowhead)"/>
  <line x1="1005" y1="299" x2="1005" y2="340" stroke="#0f172a" stroke-width="1.5" marker-end="url(#arrowhead)"/>
  <line x1="1005" y1="396" x2="1005" y2="445" stroke="#0f172a" stroke-width="1.5" marker-end="url(#arrowhead)"/>

  <!-- Middle Nodes -->
  <rect x="445" y="300" width="175" height="80" rx="10" fill="#f0f9ff" stroke="#0284c7" stroke-width="1.5"/>
  <text x="532" y="325" font-size="13" font-weight="bold" fill="#0369a1" text-anchor="middle">AMR #3 ... #N — Edge Nodes</text>
  <text x="532" y="347" font-size="11" fill="#475569" text-anchor="middle">same stack per robot</text>
  <text x="532" y="363" font-size="11" fill="#475569" text-anchor="middle">(self-contained process)</text>

  <rect x="635" y="300" width="165" height="80" rx="10" fill="#faf5ff" stroke="#9333ea" stroke-width="1.5"/>
  <text x="717" y="328" font-size="13" font-weight="bold" fill="#7e22ce" text-anchor="middle">Scenario Injector</text>
  <text x="717" y="352" font-size="11" fill="#6b21a8" text-anchor="middle">blocked aisle · robot kill</text>
  <text x="717" y="368" font-size="11" fill="#6b21a8" text-anchor="middle">task bursts</text>

  <!-- P2P Mesh Layer (Wide Pill) -->
  <rect x="365" y="445" width="510" height="64" rx="20" fill="#ffedd5" stroke="#ea580c" stroke-width="2" filter="url(#shadow)"/>
  <text x="620" y="471" font-size="14" font-weight="bold" fill="#c2410c" text-anchor="middle">P2P MESH (ZeroMQ PUB/SUB + UDP discovery)</text>
  <text x="620" y="492" font-size="12" fill="#9a3412" text-anchor="middle">pose 10 Hz · heartbeat 2 Hz · intent · reservations · auction/bids</text>

  <!-- Fleet Dashboard -->
  <rect x="410" y="580" width="420" height="78" rx="10" fill="#dcfce7" stroke="#16a34a" stroke-width="2" filter="url(#shadow)"/>
  <text x="620" y="605" font-size="14" font-weight="bold" fill="#15803d" text-anchor="middle">Fleet Dashboard</text>
  <text x="620" y="626" font-size="12" fill="#166534" text-anchor="middle">positions · battery · reserved windows · live metrics</text>
  <text x="620" y="644" font-size="12" font-weight="600" fill="#14532d" text-anchor="middle">READ-ONLY observer — never coordinates</text>

  <!-- Cross-Mesh Connectors -->
  <!-- AMR 1 to Mesh -->
  <line x1="280" y1="396" x2="460" y2="445" stroke="#0284c7" stroke-width="2" marker-end="url(#arrowhead-blue)"/>
  <rect x="300" y="405" width="115" height="18" rx="4" fill="#ffffff" stroke="#e2e8f0" stroke-width="1"/>
  <text x="357" y="418" font-size="10.5" font-weight="600" fill="#0369a1" text-anchor="middle">publish / subscribe</text>

  <!-- AMR 3 to Mesh -->
  <line x1="532" y1="380" x2="532" y2="445" stroke="#0284c7" stroke-width="2" marker-end="url(#arrowhead-blue)"/>

  <!-- Injector to Mesh -->
  <line x1="717" y1="380" x2="685" y2="445" stroke="#9333ea" stroke-width="2" stroke-dasharray="4,4" marker-end="url(#arrowhead-purple)"/>
  <rect x="705" y="405" width="55" height="18" rx="4" fill="#ffffff" stroke="#e2e8f0" stroke-width="1"/>
  <text x="732" y="418" font-size="10.5" font-weight="600" fill="#7e22ce" text-anchor="middle">injects</text>

  <!-- AMR 2 to Mesh -->
  <line x1="950" y1="396" x2="780" y2="445" stroke="#0284c7" stroke-width="2" marker-end="url(#arrowhead-blue)"/>

  <!-- Mesh to Dashboard -->
  <line x1="620" y1="509" x2="620" y2="580" stroke="#16a34a" stroke-width="2" stroke-dasharray="6,4" marker-end="url(#arrowhead-green)"/>
  <rect x="585" y="534" width="70" height="18" rx="4" fill="#ffffff" stroke="#e2e8f0" stroke-width="1"/>
  <text x="620" y="547" font-size="10.5" font-weight="600" fill="#15803d" text-anchor="middle">subscribe</text>

</svg>'''
    return svg

def main():
    os.makedirs("docs/diagrams", exist_ok=True)
    
    # 1. Excalidraw JSON
    ex_doc = build_excalidraw()
    ex_path = "docs/diagrams/architecture.excalidraw"
    with open(ex_path, "w", encoding="utf-8") as f:
        json.dump(ex_doc, f, indent=2)
    print(f"Generated Excalidraw file: {ex_path} ({len(ex_doc['elements'])} elements)")

    # 2. SVG Vector
    svg_content = build_svg()
    svg_path = "docs/diagrams/architecture.svg"
    with open(svg_path, "w", encoding="utf-8") as f:
        f.write(svg_content)
    print(f"Generated Standalone SVG: {svg_path}")

if __name__ == "__main__":
    main()
