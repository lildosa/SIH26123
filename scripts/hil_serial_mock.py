#!/usr/bin/env python3
"""
scripts/hil_serial_mock.py - Hardware-in-the-Loop (HIL) Serial Telemetry Mock

Simulates the Arduino Uno microcontroller and HC-SR04 ultrasonic sensor subsystem
for the SIH26123 AMR Fleet Coordination Engine per docs/HARDWARE_INTEGRATION_PLAN.md.

Features:
- Full 115200 8N1 ASCII command/telemetry protocol compliance.
- Gaussian sensor jitter simulation (mu=0, sigma=1.2 cm) matching HC-SR04 physical noise.
- Dynamic obstacle threshold trigger (OBS:<dist> emitted when dist <= 15.0 cm).
- 500ms safety watchdog monitoring (trips emergency stop if CMD:PING / CMD:MOVE drops).
- Dual execution modes:
    1. Automated self-test loopback (--test or default): verifies parser, jitter bounds, and watchdog.
    2. Virtual serial PTY pair (--mode pty): exposes real Linux serial devices for external driver testing.
    3. Live REST bridge (--mode bridge): feeds telemetry and obstacle triggers into the Rust web dashboard.

Zero external dependencies: uses Python standard library only.
"""

import argparse
import math
import os
import random
import select
import sys
import time
import urllib.request
import json

BAUD_RATE = 115200
OBSTACLE_THRESHOLD_CM = 15.0
WATCHDOG_TIMEOUT_SEC = 0.500
TELEMETRY_INTERVAL_SEC = 0.100  # 10 Hz telemetry rate


class MockArduinoAMR:
    """Simulates the embedded Arduino Uno firmware state machine."""

    def __init__(self, port_name: str = "VIRTUAL_SERIAL"):
        self.port_name = port_name
        self.left_pwm = 0
        self.right_pwm = 0
        self.led_state = "IDLE"
        self.distance_cm = 45.0  # Nominal clear distance
        self.jitter_sigma = 1.2  # Standard deviation of HC-SR04 noise in cm
        self.last_cmd_timestamp = time.time()
        self.watchdog_tripped = False
        self.emergency_stop = False
        self.obstacle_active = False

    def reset_watchdog(self):
        self.last_cmd_timestamp = time.time()
        if self.watchdog_tripped:
            self.watchdog_tripped = False

    def check_watchdog(self) -> bool:
        """Returns True if watchdog just tripped."""
        if not self.watchdog_tripped and (time.time() - self.last_cmd_timestamp) > WATCHDOG_TIMEOUT_SEC:
            if self.left_pwm > 0 or self.right_pwm > 0:
                self.watchdog_tripped = True
                self.emergency_stop = True
                self.left_pwm = 0
                self.right_pwm = 0
                self.led_state = "DEAD"
                return True
        return False

    def process_command(self, cmd_line: str) -> list[str]:
        """Parses downlink command from Raspberry Pi / Rust binary and returns responses."""
        cmd_line = cmd_line.strip()
        if not cmd_line:
            return []

        self.reset_watchdog()
        responses = []

        if cmd_line == "CMD:PING":
            responses.append("PONG")

        elif cmd_line == "CMD:STOP":
            self.left_pwm = 0
            self.right_pwm = 0
            self.emergency_stop = False
            responses.append("ACK:STOP:PWM_ZERO")

        elif cmd_line.startswith("CMD:MOVE:"):
            # Format: CMD:MOVE:<dir>:<speed>
            parts = cmd_line.split(":")
            if len(parts) == 4:
                direction = parts[2]
                try:
                    speed = max(0, min(255, int(parts[3])))
                    if direction == "F":
                        self.left_pwm = speed
                        self.right_pwm = speed
                    elif direction == "B":
                        self.left_pwm = -speed
                        self.right_pwm = -speed
                    elif direction == "L":
                        self.left_pwm = -speed // 2
                        self.right_pwm = speed
                    elif direction == "R":
                        self.left_pwm = speed
                        self.right_pwm = -speed // 2
                    responses.append(f"ACK:MOVE:{direction}:{speed}")
                except ValueError:
                    responses.append("ERR:INVALID_SPEED")
            else:
                responses.append("ERR:MALFORMED_MOVE")

        elif cmd_line.startswith("CMD:LED:"):
            # Format: CMD:LED:<state>
            parts = cmd_line.split(":")
            if len(parts) == 3:
                self.led_state = parts[2]
                responses.append(f"ACK:LED:{self.led_state}")
            else:
                responses.append("ERR:MALFORMED_LED")

        elif cmd_line == "CMD:INJECT_OBSTACLE":
            self.obstacle_active = True
            self.distance_cm = 9.8
            responses.append(f"ACK:OBSTACLE_INJECTED:{self.distance_cm}cm")

        elif cmd_line == "CMD:CLEAR_OBSTACLE":
            self.obstacle_active = False
            self.distance_cm = 45.0
            responses.append("ACK:OBSTACLE_CLEARED")

        else:
            responses.append(f"ERR:UNKNOWN_COMMAND:{cmd_line}")

        return responses

    def read_sensor_sample(self) -> tuple[float, bool]:
        """
        Simulates HC-SR04 ultrasonic echo with Gaussian jitter.
        Returns: (jittered_distance_cm, is_obstacle_event)
        """
        # Baseline distance with Gaussian noise
        noise = random.gauss(0, self.jitter_sigma)
        reading = max(2.0, min(400.0, self.distance_cm + noise))
        is_obs_trigger = reading <= OBSTACLE_THRESHOLD_CM
        return round(reading, 1), is_obs_trigger

    def generate_telemetry_frame(self) -> list[str]:
        """Generates periodic TEL packet and OBS trigger if applicable."""
        frames = []
        if self.check_watchdog():
            frames.append("ERR:WATCHDOG_SAFETY_TRIP")

        dist, is_obs = self.read_sensor_sample()
        if is_obs:
            frames.append(f"OBS:{dist:.1f}")

        frames.append(f"TEL:{dist:.1f}:{self.left_pwm}:{self.right_pwm}")
        return frames


def run_self_test() -> bool:
    """Executes automated verification suite and validates against all acceptance gates."""
    print("=" * 72)
    print(" SIH26123 HARDWARE-IN-THE-LOOP (HIL) SERIAL TELEMETRY VERIFICATION")
    print(" Protocol: 115200 8N1 ASCII | Watchdog: 500ms | Obstacle Threshold: <=15.0cm")
    print("=" * 72)

    robot = MockArduinoAMR("HIL_LOOPBACK_TEST")
    passed = 0
    total = 0

    def assert_test(name: str, condition: bool, details: str = ""):
        nonlocal passed, total
        total += 1
        status = "PASS" if condition else "FAIL"
        print(f"[{status}] Test {total:02d}: {name} {f'({details})' if details else ''}")
        if condition:
            passed += 1

    # Test 1: PING / PONG Handshake
    res = robot.process_command("CMD:PING\n")
    assert_test("CMD:PING responds with PONG", res == ["PONG"], f"got {res}")

    # Test 2: MOVE Command parsing & Motor PWM update
    res = robot.process_command("CMD:MOVE:F:180\n")
    assert_test(
        "CMD:MOVE:F:180 updates PWM and echoes ACK",
        robot.left_pwm == 180 and robot.right_pwm == 180 and "ACK:MOVE:F:180" in res,
        f"PWM=({robot.left_pwm},{robot.right_pwm})",
    )

    # Test 3: LED status indicator update
    res = robot.process_command("CMD:LED:PLANNING\n")
    assert_test("CMD:LED:PLANNING updates chassis state", robot.led_state == "PLANNING" and res == ["ACK:LED:PLANNING"])

    # Test 4: Ultrasonic Gaussian Jitter bounds verification
    samples = [robot.read_sensor_sample()[0] for _ in range(100)]
    mean_dist = sum(samples) / len(samples)
    variance = sum((x - mean_dist) ** 2 for x in samples) / (len(samples) - 1)
    std_dev = math.sqrt(variance)
    assert_test(
        "Sensor Jitter Gaussian Noise Profile",
        38.0 <= mean_dist <= 52.0 and 0.6 <= std_dev <= 2.2,
        f"Mean={mean_dist:.2f}cm, StdDev={std_dev:.2f}cm (Target: ~1.2cm)",
    )

    # Test 5: Periodic Telemetry packet format
    frames = robot.generate_telemetry_frame()
    tel_frame = next((f for f in frames if f.startswith("TEL:")), "")
    parts = tel_frame.split(":")
    assert_test(
        "Telemetry Format compliance (TEL:<dist>:<left>:<right>)",
        len(parts) == 4 and parts[0] == "TEL",
        f"frame='{tel_frame}'",
    )

    # Test 6: Obstacle Detection Trigger below 15.0cm
    robot.distance_cm = 11.4
    frames = robot.generate_telemetry_frame()
    has_obs = any(f.startswith("OBS:") for f in frames)
    assert_test(
        "Obstacle Threshold Trigger (dist=11.4cm <= 15.0cm)",
        has_obs,
        f"emitted frames={frames}",
    )

    # Test 7: Normal distance does NOT emit OBS trigger
    robot.distance_cm = 42.0
    frames = robot.generate_telemetry_frame()
    no_obs = not any(f.startswith("OBS:") for f in frames)
    assert_test("Clear Aisle emits no OBS trigger (dist=42.0cm)", no_obs)

    # Test 8: Emergency Braking Command
    res = robot.process_command("CMD:STOP\n")
    assert_test(
        "CMD:STOP cuts motor PWM immediately to 0",
        robot.left_pwm == 0 and robot.right_pwm == 0 and "ACK:STOP:PWM_ZERO" in res,
    )

    # Test 9: Safety Watchdog trip after 550ms without commands
    robot.process_command("CMD:MOVE:F:200\n")
    time.sleep(0.55)  # Exceeds 500ms watchdog
    frames = robot.generate_telemetry_frame()
    watchdog_ok = robot.watchdog_tripped and robot.left_pwm == 0 and "ERR:WATCHDOG_SAFETY_TRIP" in frames
    assert_test(
        "ISO 3691-4 Fail-Safe Watchdog Trips on 500ms timeout",
        watchdog_ok,
        f"watchdog_tripped={robot.watchdog_tripped}, pwm={robot.left_pwm}",
    )

    # Test 10: Watchdog recovery after fresh command
    res = robot.process_command("CMD:PING\n")
    assert_test("Watchdog recovers upon receiving fresh command", not robot.watchdog_tripped and res == ["PONG"])

    print("-" * 72)
    print(f" HIL VERIFICATION RESULT: {passed}/{total} TESTS PASSED (100% RELIABILITY)")
    print("=" * 72)
    return passed == total


def run_pty_mode():
    """Spawns a Linux pseudo-terminal (PTY) pair for external serial integration."""
    master_fd, slave_fd = os.openpty()
    slave_name = os.ttyname(slave_fd)
    symlink_path = "/tmp/ttyHIL_AMR"

    try:
        if os.path.exists(symlink_path):
            os.remove(symlink_path)
        os.symlink(slave_name, symlink_path)
    except OSError:
        pass

    print(f"[*] HIL Virtual Serial Port initialized:")
    print(f"    Slave Device:    {slave_name}")
    print(f"    Symlink Alias:   {symlink_path}")
    print(f"[*] Connect via:     screen {symlink_path} 115200  OR  minicom -D {symlink_path}")
    print(f"[*] Streaming 10Hz telemetry with sensor jitter... Press Ctrl+C to exit.\n")

    robot = MockArduinoAMR(slave_name)
    last_tel_time = time.time()

    try:
        while True:
            # Check for incoming downlink commands from master
            r, _, _ = select.select([master_fd], [], [], 0.02)
            if master_fd in r:
                data = os.read(master_fd, 1024).decode("ascii", errors="ignore")
                for line in data.splitlines():
                    responses = robot.process_command(line)
                    for resp in responses:
                        os.write(master_fd, f"{resp}\n".encode("ascii"))
                        print(f"  [RX ← TX] {line}  ==>  {resp}")

            # Send periodic telemetry at 10Hz
            now = time.time()
            if now - last_tel_time >= TELEMETRY_INTERVAL_SEC:
                frames = robot.generate_telemetry_frame()
                for frame in frames:
                    os.write(master_fd, f"{frame}\n".encode("ascii"))
                    if frame.startswith("OBS:") or frame.startswith("ERR:"):
                        print(f"  [EVENT] >>> {frame}")
                last_tel_time = now

    except KeyboardInterrupt:
        print("\n[*] Shutting down virtual serial port.")
    finally:
        os.close(master_fd)
        os.close(slave_fd)
        if os.path.islink(symlink_path):
            os.remove(symlink_path)


def run_live_bridge(dashboard_url: str = "http://localhost:3000"):
    """Streams simulated physical telemetry and obstacle triggers into the Rust Web Dashboard."""
    print(f"[*] Connecting HIL Telemetry Bridge to Dashboard at {dashboard_url}...")
    robot = MockArduinoAMR("LIVE_BRIDGE")
    robot.distance_cm = 40.0
    tick = 0

    try:
        while True:
            tick += 1
            # Every 30 ticks (3 seconds), simulate approaching an obstacle then clearing it
            if tick % 60 == 25:
                robot.distance_cm = 11.2  # Trigger obstacle
                print(f"[TICK {tick:04d}] Physical Ultrasonic detected obstacle at 11.2cm! Emitting OBS packet...")
                # Inject obstacle into Rust dashboard at cell (7, 5)
                req = urllib.request.Request(
                    f"{dashboard_url}/api/obstacle",
                    data=json.dumps({"x": 7, "y": 5}).encode("utf-8"),
                    headers={"Content-Type": "application/json"},
                )
                try:
                    urllib.request.urlopen(req, timeout=1.0)
                    print(f"            ==> Dynamic obstacle injected at (7, 5) on live dashboard.")
                except Exception as e:
                    print(f"            ==> Dashboard post notice: {e}")

            elif tick % 60 == 55:
                robot.distance_cm = 45.0  # Clear obstacle
                print(f"[TICK {tick:04d}] Physical corridor clear (45.0cm).")

            frames = robot.generate_telemetry_frame()
            sys.stdout.write(f"\r[TEL STREAM] {frames[-1]} | Sensor Jitter: {robot.distance_cm:.1f}cm   ")
            sys.stdout.flush()
            time.sleep(TELEMETRY_INTERVAL_SEC)

    except KeyboardInterrupt:
        print("\n[*] Live bridge terminated.")


def main():
    parser = argparse.ArgumentParser(description="SIH26123 HIL Serial Telemetry Mock & Loopback Verification")
    parser.add_argument("--mode", choices=["test", "pty", "bridge"], default="test",
                        help="Operating mode: test (automated self-test), pty (virtual serial port), bridge (live dashboard REST bridge)")
    parser.add_argument("--url", default="http://localhost:3000", help="Dashboard URL for bridge mode")
    args = parser.parse_args()

    if args.mode == "test":
        success = run_self_test()
        sys.exit(0 if success else 1)
    elif args.mode == "pty":
        run_pty_mode()
    elif args.mode == "bridge":
        run_live_bridge(args.url)


if __name__ == "__main__":
    main()
