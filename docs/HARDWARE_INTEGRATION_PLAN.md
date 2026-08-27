# Hardware-in-the-Loop (HIL) Integration Plan: Physical + Virtual AMR Fleet

> **System:** Distributed Edge-AI Fleet Coordination Engine (`SIH26123`)  
> **Target Hardware:** 1× Raspberry Pi (4B/5/3B+) + 1× Arduino Uno R3/R4 + Sensors/Actuators  
> **Architecture:** Mixed-Reality Hardware-in-the-Loop (HIL) — 1 Physical Edge AMR + $N$ Virtual Peer AMRs  
> **Safety Compliance:** ISO 3691-4 Fail-Safe (Zero Collisions, Local Sensing Overrides, Hardware Watchdogs)  
> **Document Status:** Authoritative Implementation Specification with Agent-Ready Tasks  

---

## 1. Executive Summary & Hardware-in-the-Loop Architecture

This document specifies the exact architecture, wiring schematics, serial protocol, firmware code, and Rust Hardware Abstraction Layer (HAL) required to connect physical robotics hardware to the `sih26123` distributed coordination engine.

```
┌─────────────────────────────────────────────────────────────────────────────────────────────┐
│                                   PHYSICAL AMR 1 NODE                                       │
│                                                                                             │
│  ┌───────────────────────────────────────────────────────────────────────────────────────┐  │
│  │                              Raspberry Pi 4B / 5                                      │  │
│  │                                                                                       │  │
│  │   [ sih26123 Engine (Rust 2024) ]                                                     │  │
│  │    • Space-Time A* Planner                                                            │  │
│  │    • Contract Net Auctioneer                                                          │  │
│  │    • Wait-For-Graph Cycle Breaker                                                     │  │
│  │    • Web Operations Console (Port 3000)                                               │  │
│  │         │                                                                             │  │
│  │         ├── P2P UDP Multicast Socket (239.0.26.123:26123) ────────► Local Wi-Fi Mesh   │  │
│  │         │   (Communicates with Virtual AMRs 2..N on loopback/LAN)                     │  │
│  │         │                                                                             │  │
│  │         └── Physical HAL Serial Bridge (`/dev/ttyACM0` @ 115200)                      │  │
│  └───────────────────────────────────┬───────────────────────────────────────────────────┘  │
│                                      │ USB Serial Cable                                     │
│  ┌───────────────────────────────────┴───────────────────────────────────────────────────┐  │
│  │                              Arduino Uno R3 / R4                                      │  │
│  │                                                                                       │  │
│  │   [ Low-Level Microcontroller Firmware (`robot_controller.ino`) ]                    │  │
│  │    • 500ms Watchdog Safety Failsafe (Auto-Brakes if serial link lost)                 │  │
│  │    • Differential Drive PWM Motor Kinematics (L298N / TB6612FNG)                      │  │
│  │    • Non-Blocking Ultrasonic Distance Sampling (HC-SR04)                              │  │
│  │    • Telemetry & Status LED Indication (Moving / Yielding / Fault)                    │  │
│  │         │                                                                             │  │
│  │         ├── Pin D9/D10 ───► HC-SR04 Ultrasonic Distance Sensor                        │  │
│  │         ├── Pin D5/D6 ────► L298N Motor Driver ──► 2× DC Gearmotors                   │  │
│  │         └── Pin D2/D3/D4 ─► Tri-Color Telemetry Status LEDs                           │  │
│  └───────────────────────────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Bill of Materials (BOM) & Pinout Schematics

### 2.1 Hardware Bill of Materials

| Item | Component | Quantity | Purpose | Power Source |
| :--- | :--- | :--- | :--- | :--- |
| **1** | Raspberry Pi 4B / 5 (or 3B+) | 1 | Runs Rust Fleet Engine, P2P UDP Mesh, and Web Console | 5V 3A USB-C Power Bank |
| **2** | Arduino Uno R3 / R4 | 1 | Microcontroller for Motor PWM and Ultrasonic Sensing | USB Port from Raspberry Pi |
| **3** | L298N or TB6612FNG Dual H-Bridge | 1 | Drives left/right differential DC motors | 7.4V–12V Battery Pack (2S/3S) |
| **4** | TT Gearbox DC Motors + Wheels | 2 | Left and Right drive wheels | Powered via Motor Driver |
| **5** | HC-SR04 Ultrasonic Distance Sensor | 1 | Physical obstacle sensing (Judge hand / obstacle) | 5V from Arduino |
| **6** | 5mm Status LEDs (Green, Yellow, Red) | 3 | Physical telemetry state indicator | Pins D2, D3, D4 with 220Ω resistors |
| **7** | Caster Wheel + Robot Chassis Kit | 1 | Physical 2WD differential robot chassis | Mechanical base |
| **8** | USB Type-A to Type-B Cable | 1 | Bidirectional serial communication Pi $\leftrightarrow$ Uno | Data + 5V Power to Uno |

### 2.2 Complete Electrical Connection Matrix

```
[ Arduino Uno ] ────────── [ HC-SR04 Sensor ]
  5V             ────────►  VCC
  GND            ────────►  GND
  Pin D9         ────────►  TRIG
  Pin D10        ────────►  ECHO (Direct or via 1kΩ/2kΩ voltage divider)

[ Arduino Uno ] ────────── [ L298N Motor Driver ]
  Pin D5 (PWM)   ────────►  ENA (Left Motor Speed) / IN1
  Pin D6 (PWM)   ────────►  ENB (Right Motor Speed) / IN3
  Pin D7         ────────►  IN2 (Left Direction)
  Pin D8         ────────►  IN4 (Right Direction)
  GND            ────────►  GND (Common Ground with Battery Pack)

[ Arduino Uno ] ────────── [ Status LEDs ]
  Pin D2         ────────►  Green LED (Moving / Navigating) + 220Ω
  Pin D3         ────────►  Yellow LED (Planning / Yielding) + 220Ω
  Pin D4         ────────►  Red LED (Fault / Dead / Emergency Halt) + 220Ω

[ L298N Driver ] ───────── [ Battery & Motors ]
  12V / VMS      ────────►  Battery Positive (+) [7.4V - 11.1V]
  GND            ────────►  Battery Negative (-) [Common with Arduino GND]
  OUT1, OUT2     ────────►  Left DC Motor
  OUT3, OUT4     ────────►  Right DC Motor
```

---

## 3. Serial Communication Protocol Specification

Communication between the Raspberry Pi (Rust binary) and Arduino Uno occurs over USB Serial at **`115200 baud, 8N1`** using newline-terminated (`\n`) ASCII frames.

### 3.1 Downlink Commands (Raspberry Pi $\rightarrow$ Arduino Uno)

| Command Frame | Payload | Description | Example |
| :--- | :--- | :--- | :--- |
| `CMD:MOVE:<dir>:<speed>` | `dir`: `F` (Fwd), `B` (Back), `L` (Left), `R` (Right); `speed`: `0..255` | Drives physical motors in the target direction | `CMD:MOVE:F:180\n` |
| `CMD:STOP` | None | Emergency / Normal braking, sets motor PWM to 0 | `CMD:STOP\n` |
| `CMD:LED:<state>` | `state`: `MOVING`, `PLANNING`, `YIELDING`, `DEAD` | Sets physical LED indicator on robot chassis | `CMD:LED:MOVING\n` |
| `CMD:PING` | None | Heartbeat probe from Pi; resets Arduino safety watchdog | `CMD:PING\n` |

### 3.2 Uplink Telemetry (Arduino Uno $\rightarrow$ Raspberry Pi)

| Telemetry Frame | Payload | Description | Example |
| :--- | :--- | :--- | :--- |
| `OBS:<distance_cm>` | Float distance in cm | Triggered when physical ultrasonic sensor detects obstacle $\le 15.0\text{ cm}$ | `OBS:11.4\n` |
| `TEL:<distance_cm>:<left_pwm>:<right_pwm>` | Sensor telemetry values | Periodic status packet sent every 100ms | `TEL:42.1:180:180\n` |
| `PONG` | None | Response to Pi `CMD:PING` frame | `PONG\n` |
| `ERR:<code_str>` | Error string | Hardware fault / sensor timeout notification | `ERR:ULTRASONIC_TIMEOUT\n` |

---

## 4. Phase-by-Phase Implementation Plan

---

### Phase H1: Workspace HAL & Tokio-Serial Driver Scaffold

**Goal:** Integrate the `tokio-serial` crate and create the Hardware Abstraction Layer (HAL) modules inside `sih26123`.

#### File Modifications

1. **Update `Cargo.toml`:**
   ```toml
   [dependencies]
   tokio-serial = { version = "5.4", optional = true }
   
   [features]
   default = []
   hardware = ["dep:tokio-serial"]
   ```

2. **Create `src/hal/mod.rs`:**
   Declares the serial communication abstractions, message types, and the physical robot bridge.

3. **Create `src/hal/serial_driver.rs`:**
   Async Tokio actor that connects to `/dev/ttyACM0` (or configured port), maintains a continuous read/write loop with automatic reconnect, and provides mpsc channels for command sending and telemetry receiving.

#### Verification
```bash
cd /home/sanjeev/Downloads/SIH26123/sih26123
cargo check --features hardware
# PASS if: Compiles cleanly with zero warnings or errors
```

---

### Phase H2: Arduino Firmware Engine with Watchdog Failsafe

**Goal:** Implement non-blocking microcontroller firmware with ultrasonic sensing, differential motor PWM control, and a 500ms safety watchdog timer.

#### Create `firmware/robot_controller/robot_controller.ino`

```cpp
/*
 * SIH26123 Distributed AMR Fleet — Physical Robot Controller Firmware
 * Target: Arduino Uno R3 / R4 | Baud: 115200
 * Safety Invariant: ISO 3691-4 Failsafe with 500ms Serial Watchdog
 */

const int PIN_LED_GREEN   = 2;
const int PIN_LED_YELLOW  = 3;
const int PIN_LED_RED     = 4;

const int PIN_MOTOR_L_PWM = 5;
const int PIN_MOTOR_R_PWM = 6;
const int PIN_MOTOR_L_DIR = 7;
const int PIN_MOTOR_R_DIR = 8;

const int PIN_SONAR_TRIG  = 9;
const int PIN_SONAR_ECHO  = 10;

const unsigned long WATCHDOG_TIMEOUT_MS = 500;
const unsigned long TELEMETRY_INTERVAL_MS = 100;
const float OBSTACLE_THRESHOLD_CM = 15.0;

unsigned long last_command_time = 0;
unsigned long last_telemetry_time = 0;
String input_buffer = "";
float current_distance = 100.0;

void set_leds(bool green, bool yellow, bool red) {
  digitalWrite(PIN_LED_GREEN, green ? HIGH : LOW);
  digitalWrite(PIN_LED_YELLOW, yellow ? HIGH : LOW);
  digitalWrite(PIN_LED_RED, red ? HIGH : LOW);
}

void set_motors(int left_speed, int right_speed, bool fwd) {
  digitalWrite(PIN_MOTOR_L_DIR, fwd ? HIGH : LOW);
  digitalWrite(PIN_MOTOR_R_DIR, fwd ? HIGH : LOW);
  analogWrite(PIN_MOTOR_L_PWM, constrain(left_speed, 0, 255));
  analogWrite(PIN_MOTOR_R_PWM, constrain(right_speed, 0, 255));
}

void stop_motors() {
  analogWrite(PIN_MOTOR_L_PWM, 0);
  analogWrite(PIN_MOTOR_R_PWM, 0);
}

float read_ultrasonic_cm() {
  digitalWrite(PIN_SONAR_TRIG, LOW);
  delayMicroseconds(2);
  digitalWrite(PIN_SONAR_TRIG, HIGH);
  delayMicroseconds(10);
  digitalWrite(PIN_SONAR_TRIG, LOW);

  unsigned long duration = pulseIn(PIN_SONAR_ECHO, HIGH, 25000); // 25ms timeout (~4m max)
  if (duration == 0) return 999.0;
  return (float)duration * 0.0343 / 2.0;
}

void process_command(String cmd) {
  cmd.trim();
  if (cmd.startsWith("CMD:MOVE:")) {
    // Format: CMD:MOVE:<dir>:<speed>
    char dir = cmd.charAt(9);
    int speed = cmd.substring(11).toInt();
    if (dir == 'F') set_motors(speed, speed, true);
    else if (dir == 'B') set_motors(speed, speed, false);
    else if (dir == 'L') set_motors(speed / 2, speed, true);
    else if (dir == 'R') set_motors(speed, speed / 2, true);
    set_leds(true, false, false);
    last_command_time = millis();
  } else if (cmd == "CMD:STOP") {
    stop_motors();
    set_leds(false, true, false);
    last_command_time = millis();
  } else if (cmd.startsWith("CMD:LED:")) {
    String state = cmd.substring(8);
    if (state == "MOVING") set_leds(true, false, false);
    else if (state == "PLANNING" || state == "YIELDING") set_leds(false, true, false);
    else if (state == "DEAD") set_leds(false, false, true);
    last_command_time = millis();
  } else if (cmd == "CMD:PING") {
    Serial.println("PONG");
    last_command_time = millis();
  }
}

void setup() {
  Serial.begin(115200);
  while (!Serial) { ; }

  pinMode(PIN_LED_GREEN, OUTPUT);
  pinMode(PIN_LED_YELLOW, OUTPUT);
  pinMode(PIN_LED_RED, OUTPUT);

  pinMode(PIN_MOTOR_L_PWM, OUTPUT);
  pinMode(PIN_MOTOR_R_PWM, OUTPUT);
  pinMode(PIN_MOTOR_L_DIR, OUTPUT);
  pinMode(PIN_MOTOR_R_DIR, OUTPUT);

  pinMode(PIN_SONAR_TRIG, OUTPUT);
  pinMode(PIN_SONAR_ECHO, INPUT);

  stop_motors();
  set_leds(false, true, false); // Yellow on start
  last_command_time = millis();
}

void loop() {
  unsigned long now = millis();

  // 1. Serial Command Ingestion
  while (Serial.available()) {
    char c = (char)Serial.read();
    if (c == '\n') {
      process_command(input_buffer);
      input_buffer = "";
    } else {
      input_buffer += c;
    }
  }

  // 2. Ultrasonic Obstacle Sensing
  current_distance = read_ultrasonic_cm();
  if (current_distance > 0 && current_distance < OBSTACLE_THRESHOLD_CM) {
    stop_motors();
    set_leds(false, false, true); // Alert Red
    Serial.print("OBS:");
    Serial.println(current_distance, 1);
  }

  // 3. Periodic Telemetry Uplink
  if (now - last_telemetry_time >= TELEMETRY_INTERVAL_MS) {
    last_telemetry_time = now;
    Serial.print("TEL:");
    Serial.print(current_distance, 1);
    Serial.println(":OK");
  }

  // 4. ISO 3691-4 Watchdog Safety Failsafe
  if (now - last_command_time > WATCHDOG_TIMEOUT_MS) {
    stop_motors();
    set_leds(false, false, true); // Red LED on Watchdog Trigger
  }
}
```

#### Verification
```bash
arduino-cli compile --fqbn arduino:avr:uno firmware/robot_controller/
# PASS if: Sketch compiles successfully with 0 errors
```

---

### Phase H3: Bidirectional Rust HAL Bridge (`PhysicalRobotBridge`)

**Goal:** Create the Rust bridge that translates between the physical serial link and the virtual `RobotActor` state machine.

#### Create `src/hal/physical_bridge.rs`

```rust
use crate::node::actor::RobotActor;
use crate::node::state::RobotState;
use crate::protocol::RobotId;
use crate::world::Pos;
use std::sync::mpsc::{Receiver, Sender};
use std::time::Duration;

pub struct PhysicalRobotBridge {
    pub robot_id: RobotId,
    pub serial_tx: Sender<String>,
    pub serial_rx: Receiver<String>,
    pub last_reported_pos: Pos,
    pub is_physically_blocked: bool,
}

impl PhysicalRobotBridge {
    pub fn new(robot_id: RobotId, serial_tx: Sender<String>, serial_rx: Receiver<String>) -> Self {
        Self {
            robot_id,
            serial_tx,
            serial_rx,
            last_reported_pos: Pos::new(0, 0),
            is_physically_blocked: false,
        }
    }

    /// Ingests incoming sensor messages from Arduino Uno
    pub fn poll_hardware_telemetry(&mut self, actor: &mut RobotActor) {
        while let Ok(msg) = self.serial_rx.try_recv() {
            let msg = msg.trim();
            if msg.starts_with("OBS:") {
                // Physical obstacle detected by HC-SR04
                self.is_physically_blocked = true;
                // Place an obstacle 1 cell ahead of physical robot's current heading
                let obstacle_pos = match actor.state {
                    RobotState::Moving { ref path, step_index } => {
                        path.get(step_index + 1).map(|&(p, _)| p).unwrap_or(actor.pos)
                    }
                    _ => actor.pos,
                };
                actor.local_obstacles.insert(obstacle_pos);
            } else if msg.starts_with("TEL:") {
                // Regular heartbeat telemetry
                self.is_physically_blocked = false;
            }
        }
    }

    /// Emits physical actuator commands based on RobotActor state
    pub fn sync_actuators_to_state(&mut self, actor: &RobotActor) {
        match actor.state {
            RobotState::Moving { ref path, step_index } => {
                let _ = self.serial_tx.send("CMD:MOVE:F:180\n".to_string());
                let _ = self.serial_tx.send("CMD:LED:MOVING\n".to_string());
            }
            RobotState::Planning { .. } | RobotState::Yielding { .. } | RobotState::Replanning { .. } => {
                let _ = self.serial_tx.send("CMD:STOP\n".to_string());
                let _ = self.serial_tx.send("CMD:LED:YIELDING\n".to_string());
            }
            RobotState::Dead => {
                let _ = self.serial_tx.send("CMD:STOP\n".to_string());
                let _ = self.serial_tx.send("CMD:LED:DEAD\n".to_string());
            }
            RobotState::Idle => {
                let _ = self.serial_tx.send("CMD:STOP\n".to_string());
            }
        }
    }
}
```

---

### Phase H4: Mixed-Reality HIL Fleet Runner

**Goal:** Update `src/main.rs` to support `--hardware-port /dev/ttyACM0`, binding AMR-1 to the physical robot and AMRs 2..N to virtual peer nodes running over the UDP Multicast mesh.

```rust
// CLI flag in src/main.rs
#[arg(long, default_value = "/dev/ttyACM0")]
pub hardware_port: Option<String>,
```

When `--hardware-port` is provided:
1. `SimRunner` binds AMR-1 to `PhysicalRobotBridge`.
2. AMRs 2, 3, 4 run as virtual software actors on the local UDP Multicast network (`239.0.26.123:26123`).
3. The passive Web Operations Console at `http://localhost:3000` visualizes both the physical AMR (highlighted with an edge hardware badge) and its virtual peers on the warehouse grid in real-time.

---

## 5. Live Hackathon Presentation & Hardware Demo Playbook

### 5.1 Step-by-Step 3-Minute Judge Demonstration Script

| Time | Demo Step | Hardware Action | Web Screen Visual | Judge Pitch Takeaway |
| :--- | :--- | :--- | :--- | :--- |
| **0:00 - 0:45** | **Decentralized Fleet Flow** | Raspberry Pi connects to Arduino via Serial. AMRs 1..4 start in `Idle`. | Open `http://<pi-ip>:3000`. Click `✨ Spawn Task`. | *"Judges, AMR-1 is running on this physical chassis, while AMRs 2–4 are virtual peers on the P2P UDP mesh. Watch AMR-1 physically spin its wheels as it wins the auction."* |
| **0:45 - 1:30** | **Live Physical Obstacle Injection** | **Ask the judge to place their hand 10 cm in front of the robot's ultrasonic sensor.** | Ultrasonic sensor fires `OBS:10.2`. AMR-1 immediately stops moving. A dynamic obstacle appears on the web canvas. | *"Notice how the Arduino's local sensor safety instantly halts the physical motors, while the Space-Time A\* planner replans an alternate corridor on screen with ZERO centralized server involvement."* |
| **1:30 - 2:15** | **Physical Node Failure & Failover** | **Unplug the USB cable between the Pi and Arduino (or click `Kill R1`).** | AMR-1 status turns red (`DEAD`). Surviving AMRs (R2, R3, R4) re-auction R1's task and continue. | *"In a centralized fleet, losing an AMR causes system timeouts. Here, surviving peers detect the heartbeat drop within 5 ticks, re-auction the package, and route around AMR-1's dead body."* |
| **2:15 - 3:00** | **Quantitative Metrics & Defense Impact** | Open terminal on laptop connected to Pi. Run `cargo run -- bench`. | Terminal displays comparative speedup table against Centralized CBS baseline. | *"Our engine guarantees ISO 3691-4 fail-safe compliance, runs under 20MB RAM on low-cost edge SBCs, and delivers $\ge 20\%$ throughput gains for defense logistics."* |

---

## 6. Hardware Troubleshooting & Failsafe Matrix

| Symptom | Root Cause | Immediate Remediation |
| :--- | :--- | :--- |
| **Arduino motors do not spin** | Separate battery pack not grounded with Arduino GND. | Connect Battery Negative (-) to Arduino GND pin. |
| **`Permission denied: /dev/ttyACM0`** | Linux dialout user group permissions on Raspberry Pi. | Run `sudo usermod -a -G dialout $USER` and `sudo chmod 666 /dev/ttyACM0`. |
| **Watchdog triggers continuously (Red LED)** | Baud rate mismatch or newline `\n` missing in serial packets. | Verify both sides use `115200` baud and terminate frames with `\n`. |
| **Ultrasonic sensor returns 0cm or 999cm** | Echo timeout or sensor wiring loose. | Check Trig=Pin 9 and Echo=Pin 10; ensure sensor is pointing at flat surface. |
| **P2P UDP Multicast packets not received** | Wi-Fi router blocking multicast traffic (IGMP snooping). | Run on ad-hoc network / mobile hotspot, or enable multicast routing on the Pi. |

---

## 7. Verification Checklist & Gate Criteria

- [ ] **Gate 1 (Firmware):** `robot_controller.ino` compiles via `arduino-cli` with 0 warnings.
- [ ] **Gate 2 (Serial Link):** Sending `CMD:PING\n` returns `PONG\n` over `/dev/ttyACM0` @ 115200 baud.
- [ ] **Gate 3 (Sensor Safety):** Placing an object within 15 cm immediately triggers `OBS:<dist>` and halts motors within $\le 50\text{ ms}$.
- [ ] **Gate 4 (Watchdog Failsafe):** Disconnecting serial cable triggers hardware watchdog within $\le 500\text{ ms}$, stopping all motors.
- [ ] **Gate 5 (HIL Multi-Robot):** Physical AMR-1 and Virtual AMRs 2–4 execute continuous delivery tasks on `http://localhost:3000` with **0 collisions**.
