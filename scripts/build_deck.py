import os
import sys
from pptx import Presentation
from pptx.util import Inches, Pt
from pptx.enum.text import PP_ALIGN, MSO_ANCHOR
from pptx.enum.shapes import MSO_SHAPE
from pptx.dml.color import RGBColor

# Palette
C_NAVY_DARK   = RGBColor(11, 25, 44)      # #0B192C Deep Slate Navy
C_NAVY_MED    = RGBColor(26, 43, 76)       # #1A2B4C
C_BLUE_ACCENT = RGBColor(0, 102, 204)     # #0066CC Vibrant Tech Blue
C_BLUE_BG     = RGBColor(224, 242, 254)   # #E0F2FE Light Blue Card
C_BLUE_BORDER = RGBColor(56, 189, 248)    # #38BDF8
C_TEAL        = RGBColor(2, 128, 144)     # #028090
C_AMBER       = RGBColor(234, 88, 12)     # #EA580C Warning/Action Orange
C_AMBER_BG    = RGBColor(255, 247, 237)   # #FFF7ED
C_AMBER_BORDER= RGBColor(253, 186, 116)   # #FDBA74
C_GREEN       = RGBColor(22, 101, 52)     # #166534 Success Green
C_GREEN_BG    = RGBColor(240, 253, 244)   # #F0FDF4
C_GREEN_BORDER= RGBColor(134, 239, 172)   # #86EFAC
C_RED         = RGBColor(185, 28, 28)     # #B91C1C Danger Red
C_RED_BG      = RGBColor(254, 242, 242)   # #FEF2F2
C_RED_BORDER  = RGBColor(252, 165, 165)   # #FCA5A5
C_WHITE       = RGBColor(255, 255, 255)
C_SLATE_BG    = RGBColor(248, 250, 252)   # #F8FAFC
C_BORDER_GRAY = RGBColor(203, 213, 225)   # #CBD5E1
C_TEXT_DARK   = RGBColor(15, 23, 42)      # #0F172A
C_TEXT_MUTED  = RGBColor(71, 85, 105)     # #475569
C_TEXT_LIGHT  = RGBColor(241, 245, 249)

prs = Presentation()
prs.slide_width = Inches(13.333)
prs.slide_height = Inches(7.5)
blank_layout = prs.slide_layouts[6]

def apply_background(slide, color=C_SLATE_BG):
    bg = slide.shapes.add_shape(MSO_SHAPE.RECTANGLE, 0, 0, Inches(13.333), Inches(7.5))
    bg.fill.solid()
    bg.fill.fore_color.rgb = color
    bg.line.fill.background()
    return bg

def add_header_footer(slide, title_text, category_text, slide_num):
    # Top header
    tb = slide.shapes.add_textbox(Inches(0.6), Inches(0.28), Inches(12.133), Inches(0.85))
    tf = tb.text_frame
    tf.word_wrap = True
    tf.margin_left = tf.margin_top = tf.margin_right = tf.margin_bottom = 0
    
    p0 = tf.paragraphs[0]
    p0.text = category_text.upper()
    p0.font.size = Pt(11)
    p0.font.bold = True
    p0.font.color.rgb = C_BLUE_ACCENT
    p0.font.name = "Calibri"
    
    p1 = tf.add_paragraph()
    p1.text = title_text
    p1.font.size = Pt(25)
    p1.font.bold = True
    p1.font.color.rgb = C_NAVY_DARK
    p1.font.name = "Arial"
    
    # Bottom footer bar
    footer_bg = slide.shapes.add_shape(MSO_SHAPE.RECTANGLE, 0, Inches(7.05), Inches(13.333), Inches(0.45))
    footer_bg.fill.solid()
    footer_bg.fill.fore_color.rgb = C_NAVY_DARK
    footer_bg.line.fill.background()
    
    fb = slide.shapes.add_textbox(Inches(0.6), Inches(7.05), Inches(11.0), Inches(0.45))
    ftf = fb.text_frame
    ftf.vertical_anchor = MSO_ANCHOR.MIDDLE
    ftf.margin_left = ftf.margin_top = ftf.margin_right = ftf.margin_bottom = 0
    fp = ftf.paragraphs[0]
    fp.text = "SIH26123 — SwarmEdge  |  Edge-AI Distributed AMR Fleet Coordination  |  Bharat Electronics Limited"
    fp.font.size = Pt(11)
    fp.font.color.rgb = C_TEXT_LIGHT
    fp.font.name = "Calibri"
    
    nb = slide.shapes.add_textbox(Inches(12.0), Inches(7.05), Inches(0.7), Inches(0.45))
    ntf = nb.text_frame
    ntf.vertical_anchor = MSO_ANCHOR.MIDDLE
    np = ntf.paragraphs[0]
    np.text = str(slide_num)
    np.alignment = PP_ALIGN.RIGHT
    np.font.size = Pt(12)
    np.font.bold = True
    np.font.color.rgb = C_TEXT_LIGHT
    np.font.name = "Calibri"

def add_card(slide, left, top, width, height, bg_color=C_WHITE, border_color=C_BORDER_GRAY):
    card = slide.shapes.add_shape(MSO_SHAPE.ROUNDED_RECTANGLE, left, top, width, height)
    card.fill.solid()
    card.fill.fore_color.rgb = bg_color
    if border_color:
        card.line.color.rgb = border_color
        card.line.width = Pt(1.5)
    else:
        card.line.fill.background()
    return card

# ==============================================================================
# SLIDE 1: COVER / PROBLEM STATEMENT OVERVIEW
# ==============================================================================
s1 = prs.slides.add_slide(blank_layout)
apply_background(s1, C_SLATE_BG)

# Header Bar
h_box = s1.shapes.add_textbox(Inches(0.8), Inches(0.45), Inches(11.7), Inches(0.7))
htf = h_box.text_frame
htf.margin_left = htf.margin_top = htf.margin_right = htf.margin_bottom = 0
hp0 = htf.paragraphs[0]
hp0.text = "SMART INDIA HACKATHON 2026"
hp0.font.size = Pt(28)
hp0.font.bold = True
hp0.font.color.rgb = C_NAVY_DARK
hp0.font.name = "Arial"

# Left Info Card (Problem Metadata)
add_card(s1, Inches(0.8), Inches(1.3), Inches(6.8), Inches(5.4), C_WHITE, C_BORDER_GRAY)
meta_box = s1.shapes.add_textbox(Inches(1.1), Inches(1.5), Inches(6.2), Inches(5.0))
mtf = meta_box.text_frame
mtf.word_wrap = True
mtf.margin_left = mtf.margin_top = mtf.margin_right = mtf.margin_bottom = 0

items = [
    ("Problem Statement ID", "SIH26123", C_BLUE_ACCENT),
    ("Organization", "Bharat Electronics Limited (BEL)", C_NAVY_DARK),
    ("Domain & Category", "Robotics and Drones  |  Software", C_NAVY_DARK),
    ("Problem Title", "Edge-AI Based Distributed Fleet Coordination for Autonomous Mobile Robots (AMRs) in Smart Warehouses", C_TEXT_DARK),
    ("Proposed Solution", "SwarmEdge — Decentralized P2P AMR Coordination Engine", C_AMBER),
    ("Hard Criteria", "Zero Inter-Robot Collisions (ISO 3691-4) & ≥20% Makespan Speedup", C_GREEN),
    ("Target Platform", "Pure Rust 2024 on Raspberry Pi 4B/5 + Arduino Uno HAL", C_NAVY_DARK),
]

for i, (label, val, val_col) in enumerate(items):
    p = mtf.paragraphs[0] if i == 0 else mtf.add_paragraph()
    p.space_after = Pt(10)
    
    r1 = p.add_run()
    r1.text = f"• {label} : "
    r1.font.bold = True
    r1.font.size = Pt(13)
    r1.font.color.rgb = C_NAVY_DARK
    r1.font.name = "Calibri"
    
    r2 = p.add_run()
    r2.text = val
    r2.font.bold = True
    r2.font.size = Pt(13)
    r2.font.color.rgb = val_col
    r2.font.name = "Calibri"

# Right Card (Architecture Visual & Impact Highlight)
add_card(s1, Inches(7.8), Inches(1.3), Inches(4.7), Inches(5.4), C_WHITE, C_BORDER_GRAY)

# Top badge in right card
rb_box = s1.shapes.add_textbox(Inches(8.1), Inches(1.55), Inches(4.1), Inches(0.4))
rtf = rb_box.text_frame
rtf.margin_left = rtf.margin_top = rtf.margin_right = rtf.margin_bottom = 0
rp = rtf.paragraphs[0]
rp.text = "DECENTRALIZED EDGE-AI FLEET ARCHITECTURE"
rp.font.bold = True
rp.font.size = Pt(12)
rp.font.color.rgb = C_BLUE_ACCENT

# Insert Architecture Image
arch_img = "docs/diagrams/architecture.png"
if os.path.exists(arch_img):
    s1.shapes.add_picture(arch_img, Inches(8.1), Inches(2.0), width=Inches(4.1))

# Bottom callout in right card
c_box = s1.shapes.add_textbox(Inches(8.1), Inches(4.4), Inches(4.1), Inches(2.1))
ctf = c_box.text_frame
ctf.word_wrap = True
ctf.margin_left = ctf.margin_top = ctf.margin_right = ctf.margin_bottom = 0

cp0 = ctf.paragraphs[0]
cp0.text = "Key Innovation Summary:"
cp0.font.bold = True
cp0.font.size = Pt(12)
cp0.font.color.rgb = C_NAVY_DARK

bullets = [
    "No Central Server: Eliminates Single Point of Failure (SPOF)",
    "Local Space-Time A*: 3D non-occupancy & edge-swap safety",
    "Dynamic Deadlock Breaker: Local Wait-For-Graph cycle DFS",
    "Market Task Auctions: Multi-factor Contract Net Protocol",
]
for b in bullets:
    bp = ctf.add_paragraph()
    bp.text = f"✔ {b}"
    bp.font.size = Pt(11)
    bp.font.color.rgb = C_TEXT_MUTED
    bp.space_after = Pt(4)

# Footer for slide 1
f1 = s1.shapes.add_shape(MSO_SHAPE.RECTANGLE, 0, Inches(7.05), Inches(13.333), Inches(0.45))
f1.fill.solid()
f1.fill.fore_color.rgb = C_NAVY_DARK
f1.line.fill.background()
fb1 = s1.shapes.add_textbox(Inches(0.8), Inches(7.05), Inches(11.7), Inches(0.45))
ftf1 = fb1.text_frame
ftf1.vertical_anchor = MSO_ANCHOR.MIDDLE
ftf1.margin_left = ftf1.margin_top = ftf1.margin_right = ftf1.margin_bottom = 0
fp1 = ftf1.paragraphs[0]
fp1.text = "SIH26123 — SwarmEdge  |  Smart India Hackathon 2026  |  Page 1"
fp1.font.size = Pt(12)
fp1.font.color.rgb = C_TEXT_LIGHT

# ==============================================================================
# SLIDE 2: PROBLEM EXISTING VS PROPOSED SOLUTION VS UVP
# ==============================================================================
s2 = prs.slides.add_slide(blank_layout)
apply_background(s2, C_SLATE_BG)
add_header_footer(s2, "Problem vs. Proposed Solution & Core Value", "System Overview", 2)

# Column 1 (Left 60%): 3 Stacked Cards (Problem, Solution, UVP)
w_col1 = Inches(7.7)

# Card 1: Problem Existing
add_card(s2, Inches(0.6), Inches(1.2), w_col1, Inches(1.75), C_RED_BG, C_RED_BORDER)
tb_p = s2.shapes.add_textbox(Inches(0.8), Inches(1.28), Inches(7.3), Inches(1.6))
tf_p = tb_p.text_frame
tf_p.word_wrap = True
tf_p.margin_left = tf_p.margin_top = tf_p.margin_right = tf_p.margin_bottom = 0
p_head = tf_p.paragraphs[0]
p_head.text = "PROBLEM EXISTING (Centralized Fleet Management Bottlenecks)"
p_head.font.bold = True
p_head.font.size = Pt(12)
p_head.font.color.rgb = C_RED

p_items = [
    ("Single Point of Failure (SPOF):", "If central server, Wi-Fi router, or dispatch kernel crashes, the entire fleet freezes."),
    ("Wi-Fi Dead-Zone Vulnerability:", "Steel racking kills RF coverage; robots lose contact, freeze mid-aisle, and stall orders."),
    ("Server Scaling Ceiling:", "Global Multi-Agent Pathfinding scales exponentially (O(V^N)), causing server saturation.")
]
for label, desc in p_items:
    p = tf_p.add_paragraph()
    p.space_after = Pt(2)
    r1 = p.add_run()
    r1.text = f"▶ {label} "
    r1.font.bold = True
    r1.font.size = Pt(10)
    r1.font.color.rgb = C_RED
    r2 = p.add_run()
    r2.text = desc
    r2.font.size = Pt(10)
    r2.font.color.rgb = C_TEXT_DARK

# Card 2: Proposed Solution
add_card(s2, Inches(0.6), Inches(3.05), w_col1, Inches(1.85), C_BLUE_BG, C_BLUE_BORDER)
tb_s = s2.shapes.add_textbox(Inches(0.8), Inches(3.13), Inches(7.3), Inches(1.7))
tf_s = tb_s.text_frame
tf_s.word_wrap = True
tf_s.margin_left = tf_s.margin_top = tf_s.margin_right = tf_s.margin_bottom = 0
s_head = tf_s.paragraphs[0]
s_head.text = "PROPOSED SOLUTION (SwarmEdge: Decentralized Edge Coordination)"
s_head.font.bold = True
s_head.font.size = Pt(12)
s_head.font.color.rgb = C_BLUE_ACCENT

s_items = [
    ("Serverless P2P Mesh:", "AMRs communicate directly over UDP Multicast (239.0.26.123:26123); dashboard is passive listener."),
    ("3D Space-Time A* Search:", "Robots locally plan (x, y, t) trajectories with strict vertex and edge-swap non-occupancy."),
    ("Dynamic Deadlock Breaker:", "Wait-For-Graph detects choke-point cycles (O(V+E) DFS); yields deterministically by priority."),
    ("Contract Net Auctions:", "Multi-factor market bidding distributes tasks based on battery, congestion, and distance.")
]
for label, desc in s_items:
    p = tf_s.add_paragraph()
    p.space_after = Pt(2)
    r1 = p.add_run()
    r1.text = f"✔ {label} "
    r1.font.bold = True
    r1.font.size = Pt(10)
    r1.font.color.rgb = C_BLUE_ACCENT
    r2 = p.add_run()
    r2.text = desc
    r2.font.size = Pt(10)
    r2.font.color.rgb = C_TEXT_DARK

# Card 3: UVP
add_card(s2, Inches(0.6), Inches(5.0), w_col1, Inches(1.85), C_GREEN_BG, C_GREEN_BORDER)
tb_u = s2.shapes.add_textbox(Inches(0.8), Inches(5.08), Inches(7.3), Inches(1.7))
tf_u = tb_u.text_frame
tf_u.word_wrap = True
tf_u.margin_left = tf_u.margin_top = tf_u.margin_right = tf_u.margin_bottom = 0
u_head = tf_u.paragraphs[0]
u_head.text = "UVP (UNIQUE VALUE PROPOSITION)"
u_head.font.bold = True
u_head.font.size = Pt(12)
u_head.font.color.rgb = C_GREEN

u_items = [
    ("Zero-Collision Guarantee (ISO 3691-4):", "Mathematical non-occupancy constraints + 50ms physical ultrasonic brake override."),
    ("Verified ≥20% Makespan Speedup:", "Measured 22.2% to 33.1% throughput gain over Centralized CBS across 2 to 8 AMRs."),
    ("Graceful Node Degradation:", "Dead AMR chassis become static obstacles; orphan tasks auto-re-auction in 5 ticks."),
    ("Ultra-Low Cost Edge HW:", "Runs on ₹5,000 Raspberry Pi + Arduino Uno; zero multi-lakh server or cloud licensing capex.")
]
for label, desc in u_items:
    p = tf_u.add_paragraph()
    p.space_after = Pt(2)
    r1 = p.add_run()
    r1.text = f"★ {label} "
    r1.font.bold = True
    r1.font.size = Pt(10)
    r1.font.color.rgb = C_GREEN
    r2 = p.add_run()
    r2.text = desc
    r2.font.size = Pt(10)
    r2.font.color.rgb = C_TEXT_DARK

# Column 2 (Right 38%): Architecture Container & Badges
w_col2 = Inches(4.3)
left_col2 = Inches(8.5)

add_card(s2, left_col2, Inches(1.2), w_col2, Inches(5.65), C_WHITE, C_BORDER_GRAY)
tb_a = s2.shapes.add_textbox(left_col2 + Inches(0.2), Inches(1.35), w_col2 - Inches(0.4), Inches(0.4))
tf_a = tb_a.text_frame
tf_a.margin_left = tf_a.margin_top = tf_a.margin_right = tf_a.margin_bottom = 0
ap = tf_a.paragraphs[0]
ap.text = "SYSTEM ARCHITECTURE FLOW"
ap.font.bold = True
ap.font.size = Pt(12)
ap.font.color.rgb = C_NAVY_DARK

if os.path.exists(arch_img):
    s2.shapes.add_picture(arch_img, left_col2 + Inches(0.15), Inches(1.75), width=w_col2 - Inches(0.3))

tb_b = s2.shapes.add_textbox(left_col2 + Inches(0.2), Inches(4.1), w_col2 - Inches(0.4), Inches(2.6))
tf_b = tb_b.text_frame
tf_b.word_wrap = True
tf_b.margin_left = tf_b.margin_top = tf_b.margin_right = tf_b.margin_bottom = 0

bp0 = tf_b.paragraphs[0]
bp0.text = "Standards & Compliance Ready:"
bp0.font.bold = True
bp0.font.size = Pt(11)
bp0.font.color.rgb = C_NAVY_DARK

badges = [
    ("ISO 3691-4 Fail-Safe:", "Local sensing halt (<50ms) + 500ms watchdog."),
    ("Pure Rust 2024:", "Zero data races, zero GC latency spikes, memory safe."),
    ("Atmanirbhar Bharat:", "Indigenous coordination layer for BEL defense depots."),
    ("Passive Listener Dashboard:", "Zero central arbitration; 100% peer autonomy.")
]
for b_lbl, b_txt in badges:
    p = tf_b.add_paragraph()
    p.space_after = Pt(4)
    r1 = p.add_run()
    r1.text = f"• {b_lbl} "
    r1.font.bold = True
    r1.font.size = Pt(10)
    r1.font.color.rgb = C_BLUE_ACCENT
    r2 = p.add_run()
    r2.text = b_txt
    r2.font.size = Pt(10)
    r2.font.color.rgb = C_TEXT_MUTED

# ==============================================================================
# SLIDE 3: TECHNICAL APPROACH & DECISION PIPELINE
# ==============================================================================
s3 = prs.slides.add_slide(blank_layout)
apply_background(s3, C_SLATE_BG)
add_header_footer(s3, "5-Phase Synchronous Edge State Machine & Pipelines", "Technical Approach", 3)

# Top 3 Pipeline Cards (Step 1 -> Step 2 -> Step 3)
w_pipe = Inches(3.9)
h_pipe = Inches(2.2)

# Pipeline 1: Task Auction
add_card(s3, Inches(0.6), Inches(1.2), w_pipe, h_pipe, C_WHITE, C_BORDER_GRAY)
tb1 = s3.shapes.add_textbox(Inches(0.75), Inches(1.3), w_pipe - Inches(0.3), h_pipe - Inches(0.2))
tf1 = tb1.text_frame
tf1.word_wrap = True
tf1.margin_left = tf1.margin_top = tf1.margin_right = tf1.margin_bottom = 0
p1 = tf1.paragraphs[0]
p1.text = "1. DECENTRALIZED AUCTION (CNP)"
p1.font.bold = True
p1.font.size = Pt(11)
p1.font.color.rgb = C_BLUE_ACCENT

pipe1_lines = [
    "• Trigger: New task broadcast over UDP mesh",
    "• Cost Function: Distance + Congestion + Battery + Delay + Urgency",
    "• Bidding Window: 5-tick decentralized collection",
    "• Award: Deterministic lowest cost; Robot ID tiebreak"
]
for l in pipe1_lines:
    p = tf1.add_paragraph()
    p.text = l
    p.font.size = Pt(9.5)
    p.font.color.rgb = C_TEXT_DARK
    p.space_after = Pt(2)

# Pipeline 2: Space-Time Planning
add_card(s3, Inches(4.7), Inches(1.2), w_pipe, h_pipe, C_WHITE, C_BORDER_GRAY)
tb2 = s3.shapes.add_textbox(Inches(4.85), Inches(1.3), w_pipe - Inches(0.3), h_pipe - Inches(0.2))
tf2 = tb2.text_frame
tf2.word_wrap = True
tf2.margin_left = tf2.margin_top = tf2.margin_right = tf2.margin_bottom = 0
p2 = tf2.paragraphs[0]
p2.text = "2. SPACE-TIME A* COLLISION AVOIDANCE"
p2.font.bold = True
p2.font.size = Pt(11)
p2.font.color.rgb = C_TEAL

pipe2_lines = [
    "• 3D Search: Plans path across (x, y, t) dimensions",
    "• Non-Occupancy: Vertex reservation table check",
    "• Swap Safety: Prevents (u,v) <-> (v,u) reverse crossing",
    "• In-Place Waiting: Temporal wait moves before replanning"
]
for l in pipe2_lines:
    p = tf2.add_paragraph()
    p.text = l
    p.font.size = Pt(9.5)
    p.font.color.rgb = C_TEXT_DARK
    p.space_after = Pt(2)

# Pipeline 3: Deadlock Breaking
add_card(s3, Inches(8.8), Inches(1.2), w_pipe, h_pipe, C_WHITE, C_BORDER_GRAY)
tb3 = s3.shapes.add_textbox(Inches(8.95), Inches(1.3), w_pipe - Inches(0.3), h_pipe - Inches(0.2))
tf3 = tb3.text_frame
tf3.word_wrap = True
tf3.margin_left = tf3.margin_top = tf3.margin_right = tf3.margin_bottom = 0
p3 = tf3.paragraphs[0]
p3.text = "3. DEADLOCK BREAKING (WFG DFS)"
p3.font.bold = True
p3.font.size = Pt(11)
p3.font.color.rgb = C_AMBER

pipe3_lines = [
    "• Local Graph: Wait-For-Graph of blocking dependencies",
    "• Cycle Detection: O(V + E) DFS finds circular wait loops",
    "• Right-of-Way: Lower priority number yields cleanly",
    "• Tiebreak: Lowest Robot ID backs off & reroutes"
]
for l in pipe3_lines:
    p = tf3.add_paragraph()
    p.text = l
    p.font.size = Pt(9.5)
    p.font.color.rgb = C_TEXT_DARK
    p.space_after = Pt(2)

# Bottom Section: 5-Phase Loop (Left 60%) + Tech Stack (Right 40%)
add_card(s3, Inches(0.6), Inches(3.55), Inches(7.7), Inches(3.3), C_WHITE, C_BORDER_GRAY)
tb_loop = s3.shapes.add_textbox(Inches(0.8), Inches(3.7), Inches(7.3), Inches(3.0))
tf_loop = tb_loop.text_frame
tf_loop.word_wrap = True
tf_loop.margin_left = tf_loop.margin_top = tf_loop.margin_right = tf_loop.margin_bottom = 0

lp = tf_loop.paragraphs[0]
lp.text = "5-PHASE SYNCHRONOUS EDGE TICK CYCLE (RobotActor Loop)"
lp.font.bold = True
lp.font.size = Pt(12)
lp.font.color.rgb = C_NAVY_DARK

phases = [
    ("Phase 1: SENSE", "Local LiDAR/ultrasonic scans 3-cell radius for dynamic obstacles & peers; updates local map."),
    ("Phase 2: DECIDE", "Drains inbox, dedup via seq numbers, updates peer poses, detects WFG cycles, bids/awards auctions."),
    ("Phase 3: FLUSH/DELIVER", "Broadcasts queued envelopes (Pose, Intent, Bids, Conflicts) over UDP multicast mesh."),
    ("Phase 4: MOVE", "Advances position along planned (x, y, t) trajectory; triggers HAL motor pulses."),
    ("Phase 5: EVALUATE", "Publishes telemetry to passive dashboard watch channel; verifies invariant zero-collision pass.")
]
for name, detail in phases:
    p = tf_loop.add_paragraph()
    p.space_after = Pt(4)
    r1 = p.add_run()
    r1.text = f"▶ {name} — "
    r1.font.bold = True
    r1.font.size = Pt(10)
    r1.font.color.rgb = C_BLUE_ACCENT
    r2 = p.add_run()
    r2.text = detail
    r2.font.size = Pt(10)
    r2.font.color.rgb = C_TEXT_DARK

# Right Card: Tech Stack & HIL HAL
add_card(s3, Inches(8.5), Inches(3.55), Inches(4.2), Inches(3.3), C_WHITE, C_BORDER_GRAY)
tb_stack = s3.shapes.add_textbox(Inches(8.7), Inches(3.7), Inches(3.8), Inches(3.0))
tf_stack = tb_stack.text_frame
tf_stack.word_wrap = True
tf_stack.margin_left = tf_stack.margin_top = tf_stack.margin_right = tf_stack.margin_bottom = 0

sp = tf_stack.paragraphs[0]
sp.text = "HARDWARE & SOFTWARE TECH STACK"
sp.font.bold = True
sp.font.size = Pt(12)
sp.font.color.rgb = C_NAVY_DARK

stack_items = [
    ("Core Engine:", "Rust 1.85+ (2024 Edition) — Memory Safe, No GC"),
    ("Concurrency:", "Tokio async runtime, Watch/Broadcast channels"),
    ("Serialization:", "Serde / JSON Envelope with Monotonic SeqNum"),
    ("Web Console:", "Axum WebSockets + HTML5 Canvas UI"),
    ("P2P Mesh:", "UDP Multicast (239.0.26.123:26123)"),
    ("Microcontroller:", "Arduino Uno (C++11, L298N PWM, HC-SR04)"),
    ("Safety Watchdog:", "500ms Serial Timeout Auto-Brake (ISO 3691-4)")
]
for comp, desc in stack_items:
    p = tf_stack.add_paragraph()
    p.space_after = Pt(3)
    r1 = p.add_run()
    r1.text = f"• {comp} "
    r1.font.bold = True
    r1.font.size = Pt(10)
    r1.font.color.rgb = C_TEAL
    r2 = p.add_run()
    r2.text = desc
    r2.font.size = Pt(9.5)
    r2.font.color.rgb = C_TEXT_MUTED

# ==============================================================================
# SLIDE 4: FEASIBILITY AND VIABILITY
# ==============================================================================
s4 = prs.slides.add_slide(blank_layout)
apply_background(s4, C_SLATE_BG)
add_header_footer(s4, "Feasibility, Viability & Risk Mitigations", "Feasibility & Viability", 4)

w_box4 = Inches(5.9)
h_top4 = Inches(3.1)

# Card 1: Feasibility
add_card(s4, Inches(0.6), Inches(1.2), w_box4, h_top4, C_WHITE, C_BLUE_BORDER)
tb_f = s4.shapes.add_textbox(Inches(0.8), Inches(1.3), w_box4 - Inches(0.4), h_top4 - Inches(0.2))
tf_f = tb_f.text_frame
tf_f.word_wrap = True
tf_f.margin_left = tf_f.margin_top = tf_f.margin_right = tf_f.margin_bottom = 0

fp = tf_f.paragraphs[0]
fp.text = "FEASIBILITY (Technical, Operational & Economic)"
fp.font.bold = True
fp.font.size = Pt(12)
fp.font.color.rgb = C_BLUE_ACCENT

f_items = [
    ("Technical Maturity:", "Implemented in pure Rust 2024; 49 integration tests across 11 suites pass with 100% reliability."),
    ("Operational Independence:", "Operates in total radio shadow; peer drop triggers 5-tick dead-chassis conversion and task re-auction."),
    ("Safety Compliance:", "ISO 3691-4 industrial safety standard; <50ms ultrasonic emergency brake; 500ms hardware watchdog."),
    ("Economic Feasibility:", "Runs on ₹5,000 Raspberry Pi 4B/5 + Arduino Uno; eliminates multi-lakh central server capex.")
]
for lbl, val in f_items:
    p = tf_f.add_paragraph()
    p.space_after = Pt(4)
    r1 = p.add_run()
    r1.text = f"✔ {lbl} "
    r1.font.bold = True
    r1.font.size = Pt(10)
    r1.font.color.rgb = C_NAVY_DARK
    r2 = p.add_run()
    r2.text = val
    r2.font.size = Pt(10)
    r2.font.color.rgb = C_TEXT_MUTED

# Card 2: Viability
add_card(s4, Inches(6.8), Inches(1.2), w_box4, h_top4, C_WHITE, C_GREEN_BORDER)
tb_v = s4.shapes.add_textbox(Inches(7.0), Inches(1.3), w_box4 - Inches(0.4), h_top4 - Inches(0.2))
tf_v = tb_v.text_frame
tf_v.word_wrap = True
tf_v.margin_left = tf_v.margin_top = tf_v.margin_right = tf_v.margin_bottom = 0

vp = tf_v.paragraphs[0]
vp.text = "VIABILITY (Market, Defense Sovereignty & ROI)"
vp.font.bold = True
vp.font.size = Pt(12)
vp.font.color.rgb = C_GREEN

v_items = [
    ("Market Opportunity:", "India AMR market growing $92.6M (2026) -> $245M (2031) at 17.6% CAGR; India warehousing reaches $181B by 2035."),
    ("Strategic (BEL / Defense):", "Indigenous Atmanirbhar coordination engine; breaks dependence on foreign FMS (OTTO/MiR/Geek+)."),
    ("Measurable ROI:", "≥20% throughput boost on existing robot fleets with zero additional hardware spend."),
    ("Scalability Model:", "Communication scales with local neighbors O(local), avoiding the O(N^2) server saturation ceiling.")
]
for lbl, val in v_items:
    p = tf_v.add_paragraph()
    p.space_after = Pt(4)
    r1 = p.add_run()
    r1.text = f"★ {lbl} "
    r1.font.bold = True
    r1.font.size = Pt(10)
    r1.font.color.rgb = C_NAVY_DARK
    r2 = p.add_run()
    r2.text = val
    r2.font.size = Pt(10)
    r2.font.color.rgb = C_TEXT_MUTED

# Bottom 2 Cards: Challenges & Mitigations
h_bot4 = Inches(2.3)

# Technical Challenges
add_card(s4, Inches(0.6), Inches(4.5), w_box4, h_bot4, C_WHITE, C_BORDER_GRAY)
tb_tc = s4.shapes.add_textbox(Inches(0.8), Inches(4.6), w_box4 - Inches(0.4), h_bot4 - Inches(0.2))
tf_tc = tb_tc.text_frame
tf_tc.word_wrap = True
tf_tc.margin_left = tf_tc.margin_top = tf_tc.margin_right = tf_tc.margin_bottom = 0

tcp = tf_tc.paragraphs[0]
tcp.text = "TECHNICAL CHALLENGES & MITIGATIONS"
tcp.font.bold = True
tcp.font.size = Pt(11)
tcp.font.color.rgb = C_NAVY_DARK

tc_lines = [
    ("Risk: UDP Packet Loss in Steel Warehouse Shadows", C_RED),
    ("Mitigation: Monotonic sequence tracking rejects duplicates; periodic 1-tick soft-state heartbeats ensure self-healing telemetry; local ultrasonic HAL triggers emergency stop (<50ms) regardless of network status.", C_TEXT_MUTED)
]
p1 = tf_tc.add_paragraph()
p1.text = f"• {tc_lines[0][0]}"
p1.font.bold = True
p1.font.size = Pt(10)
p1.font.color.rgb = tc_lines[0][1]
p1.space_after = Pt(2)
p2 = tf_tc.add_paragraph()
p2.text = f"  {tc_lines[1][0]}"
p2.font.size = Pt(9.5)
p2.font.color.rgb = tc_lines[1][1]

# Operational Challenges
add_card(s4, Inches(6.8), Inches(4.5), w_box4, h_bot4, C_WHITE, C_BORDER_GRAY)
tb_oc = s4.shapes.add_textbox(Inches(7.0), Inches(4.6), w_box4 - Inches(0.4), h_bot4 - Inches(0.2))
tf_oc = tb_oc.text_frame
tf_oc.word_wrap = True
tf_oc.margin_left = tf_oc.margin_top = tf_oc.margin_right = tf_oc.margin_bottom = 0

ocp = tf_oc.paragraphs[0]
ocp.text = "OPERATIONAL & SAFETY CHALLENGES & MITIGATIONS"
ocp.font.bold = True
ocp.font.size = Pt(11)
ocp.font.color.rgb = C_NAVY_DARK

oc_lines = [
    ("Risk: Circular Deadlock at High-Traffic 4-Way Intersections", C_AMBER),
    ("Mitigation: Onboard Wait-For-Graph DFS detects circular wait cycles in O(V+E) time; deterministic priority rules and Robot ID tiebreakers force lower priority robot to yield and reroute without central intervention.", C_TEXT_MUTED)
]
p3 = tf_oc.add_paragraph()
p3.text = f"• {oc_lines[0][0]}"
p3.font.bold = True
p3.font.size = Pt(10)
p3.font.color.rgb = oc_lines[0][1]
p3.space_after = Pt(2)
p4 = tf_oc.add_paragraph()
p4.text = f"  {oc_lines[1][0]}"
p4.font.size = Pt(9.5)
p4.font.color.rgb = oc_lines[1][1]

# ==============================================================================
# SLIDE 5: IMPACTS, BENEFITS & HOW IT WORKS (SCENARIOS)
# ==============================================================================
s5 = prs.slides.add_slide(blank_layout)
apply_background(s5, C_SLATE_BG)
add_header_footer(s5, "Impacts, Benchmarks & 3 Operational Scenarios", "Impacts & Benefits", 5)

# Top Left: Impact Pillars (Width: 6.8 Inches)
add_card(s5, Inches(0.6), Inches(1.2), Inches(6.8), Inches(2.55), C_WHITE, C_BORDER_GRAY)
tb_imp = s5.shapes.add_textbox(Inches(0.8), Inches(1.3), Inches(6.4), Inches(2.35))
tf_imp = tb_imp.text_frame
tf_imp.word_wrap = True
tf_imp.margin_left = tf_imp.margin_top = tf_imp.margin_right = tf_imp.margin_bottom = 0

impp = tf_imp.paragraphs[0]
impp.text = "CORE IMPACT PILLARS (Quantified & Verified)"
impp.font.bold = True
impp.font.size = Pt(12)
impp.font.color.rgb = C_NAVY_DARK

imp_items = [
    ("Operational Resilience:", "100% Graceful Degradation. Killing any AMR leaves the surviving fleet operational; tasks re-auctioned within 5 ticks with zero central server intervention."),
    ("Productivity & Throughput:", "Measured 22.2% to 33.1% makespan speedup over Centralized CBS dispatcher across 2 to 8 AMRs; zero bottleneck idle freezes."),
    ("Defense & Strategic Sovereignty:", "Air-gapped edge operation with zero cloud exposure, ideal for high-security defense depots (BEL), ordnance factories, and naval dockyards.")
]
for lbl, desc in imp_items:
    p = tf_imp.add_paragraph()
    p.space_after = Pt(3)
    r1 = p.add_run()
    r1.text = f"• {lbl} "
    r1.font.bold = True
    r1.font.size = Pt(10)
    r1.font.color.rgb = C_BLUE_ACCENT
    r2 = p.add_run()
    r2.text = desc
    r2.font.size = Pt(9.5)
    r2.font.color.rgb = C_TEXT_MUTED

# Top Right: Comparison Table (Width: 5.3 Inches)
tb_rows = 6
tb_cols = 3
left_tbl = Inches(7.6)
top_tbl = Inches(1.2)
w_tbl = Inches(5.1)
h_tbl = Inches(2.55)

table_shape = s5.shapes.add_table(tb_rows, tb_cols, left_tbl, top_tbl, w_tbl, h_tbl)
table = table_shape.table
table.columns[0].width = Inches(1.7)
table.columns[1].width = Inches(1.7)
table.columns[2].width = Inches(1.7)

headers = ["Feature", "Centralized FMS", "SwarmEdge (Ours)"]
for col_idx, h in enumerate(headers):
    cell = table.cell(0, col_idx)
    cell.text = h
    cell.fill.solid()
    cell.fill.fore_color.rgb = C_NAVY_DARK
    p = cell.text_frame.paragraphs[0]
    p.font.bold = True
    p.font.size = Pt(10)
    p.font.color.rgb = C_WHITE
    p.alignment = PP_ALIGN.CENTER

table_data = [
    ("Coordination Model", "Central Server (SPOF)", "Decentralized P2P Mesh"),
    ("Wi-Fi Drop Behavior", "Entire Fleet Halts", "Continues Local Operation"),
    ("Deadlock Breaking", "Manual / Central Freeze", "Autonomous WFG DFS Cycle Breaker"),
    ("Hardware Cost", "High-End Server + Licenses", "₹5,000 Edge Board (RPi+Arduino)"),
    ("Safety Invariant", "Probabilistic / Network-reliant", "100% Zero Collisions (ISO 3691-4)")
]

for row_idx, row_vals in enumerate(table_data):
    for col_idx, val in enumerate(row_vals):
        cell = table.cell(row_idx + 1, col_idx)
        cell.text = val
        cell.fill.solid()
        cell.fill.fore_color.rgb = C_WHITE if row_idx % 2 == 0 else C_SLATE_BG
        p = cell.text_frame.paragraphs[0]
        p.font.size = Pt(9)
        p.font.name = "Calibri"
        if col_idx == 0:
            p.font.bold = True
            p.font.color.rgb = C_NAVY_DARK
        elif col_idx == 1:
            p.font.color.rgb = C_RED
        else:
            p.font.bold = True
            p.font.color.rgb = C_GREEN

# Bottom Section: 3 Demo Scenarios (Cards)
w_scen = Inches(3.9)
h_scen = Inches(2.8)

# Scenario 1: Choke-Point
add_card(s5, Inches(0.6), Inches(3.95), w_scen, h_scen, C_WHITE, C_BLUE_BORDER)
tb_s1 = s5.shapes.add_textbox(Inches(0.75), Inches(4.05), w_scen - Inches(0.3), h_scen - Inches(0.2))
tf_s1 = tb_s1.text_frame
tf_s1.word_wrap = True
tf_s1.margin_left = tf_s1.margin_top = tf_s1.margin_right = tf_s1.margin_bottom = 0

s1_p = tf_s1.paragraphs[0]
s1_p.text = "SCENARIO 1: CHOKE-POINT NEGOTIATION"
s1_p.font.bold = True
s1_p.font.size = Pt(11)
s1_p.font.color.rgb = C_BLUE_ACCENT

s1_lines = [
    ("Condition:", "2 AMRs converge head-on in 1-lane narrow corridor."),
    ("Mechanism:", "Space-Time A* checks peer intent; lower priority robot executes temporal wait in alcove; higher priority robot clears corridor."),
    ("Time / Result:", "32 ticks  |  0 Collisions  |  0 Deadlocks"),
    ("Best For:", "Narrow storage aisles, conveyor junctions, one-way ramps.")
]
for l_name, l_val in s1_lines:
    p = tf_s1.add_paragraph()
    p.space_after = Pt(2)
    r1 = p.add_run()
    r1.text = f"• {l_name} "
    r1.font.bold = True
    r1.font.size = Pt(9.5)
    r1.font.color.rgb = C_NAVY_DARK
    r2 = p.add_run()
    r2.text = l_val
    r2.font.size = Pt(9)
    r2.font.color.rgb = C_TEXT_MUTED

# Scenario 2: Blocked Aisle
add_card(s5, Inches(4.7), Inches(3.95), w_scen, h_scen, C_WHITE, C_AMBER_BORDER)
tb_s2 = s5.shapes.add_textbox(Inches(4.85), Inches(4.05), w_scen - Inches(0.3), h_scen - Inches(0.2))
tf_s2 = tb_s2.text_frame
tf_s2.word_wrap = True
tf_s2.margin_left = tf_s2.margin_top = tf_s2.margin_right = tf_s2.margin_bottom = 0

s2_p = tf_s2.paragraphs[0]
s2_p.text = "SCENARIO 2: DYNAMIC OBSTACLE RE-ROUTE"
s2_p.font.bold = True
s2_p.font.size = Pt(11)
s2_p.font.color.rgb = C_AMBER

s2_lines = [
    ("Condition:", "Fallen pallet or human worker blocks primary route."),
    ("Mechanism:", "Local 3-cell sense interface detects obstacle; robot marks cell as dynamic obstacle and triggers instant onboard replanning."),
    ("Time / Result:", "Instantaneous replan (<10ms)  |  0 Collision halts"),
    ("Best For:", "Dynamic fulfillment floors, mixed human-robot zones.")
]
for l_name, l_val in s2_lines:
    p = tf_s2.add_paragraph()
    p.space_after = Pt(2)
    r1 = p.add_run()
    r1.text = f"• {l_name} "
    r1.font.bold = True
    r1.font.size = Pt(9.5)
    r1.font.color.rgb = C_NAVY_DARK
    r2 = p.add_run()
    r2.text = l_val
    r2.font.size = Pt(9)
    r2.font.color.rgb = C_TEXT_MUTED

# Scenario 3: Robot Kill
add_card(s5, Inches(8.8), Inches(3.95), w_scen, h_scen, C_WHITE, C_GREEN_BORDER)
tb_s3 = s5.shapes.add_textbox(Inches(8.95), Inches(4.05), w_scen - Inches(0.3), h_scen - Inches(0.2))
tf_s3 = tb_s3.text_frame
tf_s3.word_wrap = True
tf_s3.margin_left = tf_s3.margin_top = tf_s3.margin_right = tf_s3.margin_bottom = 0

s3_p = tf_s3.paragraphs[0]
s3_p.text = "SCENARIO 3: GRACEFUL NODE KILL RECOVERY"
s3_p.font.bold = True
s3_p.font.size = Pt(11)
s3_p.font.color.rgb = C_GREEN

s3_lines = [
    ("Condition:", "AMR suffers hardware fault or battery death mid-mission."),
    ("Mechanism:", "Surviving peers miss heartbeats (>5 ticks); dead robot pose converted to static obstacle; orphan task auto-broadcasted to auction pool."),
    ("Time / Result:", "Re-awarded in 5 ticks  |  Fleet continues seamlessly"),
    ("Best For:", "Uninterrupted 24/7 mission-critical defense operations.")
]
for l_name, l_val in s3_lines:
    p = tf_s3.add_paragraph()
    p.space_after = Pt(2)
    r1 = p.add_run()
    r1.text = f"• {l_name} "
    r1.font.bold = True
    r1.font.size = Pt(9.5)
    r1.font.color.rgb = C_NAVY_DARK
    r2 = p.add_run()
    r2.text = l_val
    r2.font.size = Pt(9)
    r2.font.color.rgb = C_TEXT_MUTED

# ==============================================================================
# SLIDE 6: RESEARCH, REFERENCES & PROJECT DELIVERABLES
# ==============================================================================
s6 = prs.slides.add_slide(blank_layout)
apply_background(s6, C_SLATE_BG)
add_header_footer(s6, "Research References, Differentiation & Deliverables", "Research & Resources", 6)

# Top Card: Academic Foundations & Standards (Width: 12.1 Inches)
add_card(s6, Inches(0.6), Inches(1.2), Inches(12.133), Inches(1.75), C_WHITE, C_BORDER_GRAY)
tb_ref = s6.shapes.add_textbox(Inches(0.8), Inches(1.28), Inches(11.7), Inches(1.6))
tf_ref = tb_ref.text_frame
tf_ref.word_wrap = True
tf_ref.margin_left = tf_ref.margin_top = tf_ref.margin_right = tf_ref.margin_bottom = 0

ref_p = tf_ref.paragraphs[0]
ref_p.text = "ACADEMIC FOUNDATIONS & SAFETY STANDARDS"
ref_p.font.bold = True
ref_p.font.size = Pt(12)
ref_p.font.color.rgb = C_NAVY_DARK

refs = [
    ("Cooperative Pathfinding:", "Silver (AIIDE 2005) — Space-Time Reservation Tables and Windowed Hierarchical Planning."),
    ("Optimal MAPF Baseline:", "Sharon, Stern, Felner, Sturtevant (AAAI 2012 / AIJ 2015) — Conflict-Based Search (CBS) for Multi-Agent Pathfinding."),
    ("Market Task Allocation:", "Smith (IEEE TC 1980) — Contract Net Protocol & Gerkey/Matarić (IEEE T-RA 2002) MURDOCH Multi-Robot Auctions."),
    ("Safety & Interoperability Standards:", "ISO 3691-4:2023 (Safety of Driverless Industrial Trucks) and VDA 5050 (AGV/AMR Interface Standard).")
]
for lbl, txt in refs:
    p = tf_ref.add_paragraph()
    p.space_after = Pt(2)
    r1 = p.add_run()
    r1.text = f"• {lbl} "
    r1.font.bold = True
    r1.font.size = Pt(10)
    r1.font.color.rgb = C_BLUE_ACCENT
    r2 = p.add_run()
    r2.text = txt
    r2.font.size = Pt(10)
    r2.font.color.rgb = C_TEXT_MUTED

# Middle Section: Solutions Already Exist vs Ours (2 Columns)
w_half = Inches(5.9)
h_mid6 = Inches(1.8)

# Left: Solutions Already Exist
add_card(s6, Inches(0.6), Inches(3.1), w_half, h_mid6, C_WHITE, C_BORDER_GRAY)
tb_ex = s6.shapes.add_textbox(Inches(0.8), Inches(3.2), w_half - Inches(0.4), h_mid6 - Inches(0.2))
tf_ex = tb_ex.text_frame
tf_ex.word_wrap = True
tf_ex.margin_left = tf_ex.margin_top = tf_ex.margin_right = tf_ex.margin_bottom = 0

ep = tf_ex.paragraphs[0]
ep.text = "SOLUTIONS ALREADY EXIST (Centralized & Proprietary)"
ep.font.bold = True
ep.font.size = Pt(11)
ep.font.color.rgb = C_RED

exist_lines = [
    "• Commercial FMS (OTTO Motors, MiR Fleet, Geek+, Locus): Centralized dispatch; single point of failure; vendor lock-in.",
    "• Open Source / Standards (openTCS, Open-RMF, VDA 5050): Centralized routing kernel; fails when Wi-Fi disconnects.",
    "• Academic Decentralized Papers: Idealized zero-latency simulation only; no physical hardware deployment."
]
for l in exist_lines:
    p = tf_ex.add_paragraph()
    p.text = l
    p.font.size = Pt(9.5)
    p.font.color.rgb = C_TEXT_MUTED
    p.space_after = Pt(2)

# Right: Our Solution Stands Out
add_card(s6, Inches(6.8), Inches(3.1), w_half, h_mid6, C_WHITE, C_GREEN_BORDER)
tb_out = s6.shapes.add_textbox(Inches(7.0), Inches(3.2), w_half - Inches(0.4), h_mid6 - Inches(0.2))
tf_out = tb_out.text_frame
tf_out.word_wrap = True
tf_out.margin_left = tf_out.margin_top = tf_out.margin_right = tf_out.margin_bottom = 0

op = tf_out.paragraphs[0]
op.text = "OUR SOLUTION STANDS OUT (SwarmEdge)"
op.font.bold = True
op.font.size = Pt(11)
op.font.color.rgb = C_GREEN

out_lines = [
    "• True P2P Serverless Mesh: Zero central infrastructure; operates reliably in RF dead zones.",
    "• Metric-Proven Advantage: +22.2% to +33.1% makespan speedup over CBS baseline with 0 collisions.",
    "• Self-Healing Fault Tolerance: Automated deadlock cycle breaking and lost-peer task re-auctions.",
    "• Production Edge & HIL: Runs in pure Rust 2024 on ₹5,000 Raspberry Pi + Arduino Uno hardware."
]
for l in out_lines:
    p = tf_out.add_paragraph()
    p.text = l
    p.font.size = Pt(9.5)
    p.font.color.rgb = C_TEXT_DARK
    p.space_after = Pt(2)

# Bottom Card: Project Deliverables & Verifiable Artifacts
add_card(s6, Inches(0.6), Inches(5.05), Inches(12.133), Inches(1.8), C_WHITE, C_BORDER_GRAY)
tb_del = s6.shapes.add_textbox(Inches(0.8), Inches(5.15), Inches(11.7), Inches(1.6))
tf_del = tb_del.text_frame
tf_del.word_wrap = True
tf_del.margin_left = tf_del.margin_top = tf_del.margin_right = tf_del.margin_bottom = 0

dp = tf_del.paragraphs[0]
dp.text = "VERIFIABLE PROJECT ARTIFACTS & DELIVERABLES"
dp.font.bold = True
dp.font.size = Pt(11)
dp.font.color.rgb = C_NAVY_DARK

delivs = [
    ("Interactive Web Operations Console:", "Real-time telemetry on port 3000 (Axum + WebSockets + HTML5 Canvas) with dynamic wall injection, custom task dispatch, and AMR kill/restore tools."),
    ("Automated Test Suite (100% Pass):", "49 passing integration tests across 11 test suites covering auctions, deadlocks, packet loss, sequence dedup, and CBS comparisons (`make test`)."),
    ("Comparative Benchmark Suite:", "Single-command reproducible benchmark pipeline (`make bench`) evaluating distributed engine vs Centralized CBS across 2, 4, 6, and 8 AMRs."),
    ("Hardware-in-the-Loop Integration Plan:", "Complete Raspberry Pi + Arduino Uno wiring schematics, serial protocol, and C++ firmware in docs/HARDWARE_INTEGRATION_PLAN.md.")
]
for lbl, txt in delivs:
    p = tf_del.add_paragraph()
    p.space_after = Pt(2)
    r1 = p.add_run()
    r1.text = f"✔ {lbl} "
    r1.font.bold = True
    r1.font.size = Pt(9.5)
    r1.font.color.rgb = C_BLUE_ACCENT
    r2 = p.add_run()
    r2.text = txt
    r2.font.size = Pt(9.5)
    r2.font.color.rgb = C_TEXT_MUTED

# Save presentation
output_path = "SIH26123_SwarmEdge.pptx"
prs.save(output_path)
print(f"Presentation saved successfully to {output_path}")
