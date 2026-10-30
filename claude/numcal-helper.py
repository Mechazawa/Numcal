#!/usr/bin/env python3
"""Delivers NumCal key presses to the Ghostty terminal of the Claude Code session shown on the device.

The device sends `keys\t<session>\t<strokes>` and `jump\t<session>` lines on its Claude serial port.
A session is matched to a terminal through the terminal's foreground pid and the session id Claude Code
records for that pid in ~/.claude/sessions/<pid>.json, so a reused tty or folder never selects the
wrong terminal. Without a match nothing is sent. Runs as a launchd agent, see nl.mechazawa.numcal-helper.plist.
"""

import json
import os
import re
import select
import subprocess
import sys
import termios
import time
from pathlib import Path

PORT = "/dev/cu.usbmodem123456783"
SESSIONS = Path(os.environ.get("NUMCAL_SESSIONS_DIR", Path.home() / ".claude" / "sessions"))
HEARTBEAT_SECONDS = 2
STROKE = re.compile(r"^(?:(control|shift|control,shift):)?([A-Za-z0-9]+)$")


def log(message):
    print(time.strftime("%H:%M:%S"), message, flush=True)


def claude_pids(session):
    """Pids of running Claude Code processes whose session id starts with the device's short id."""
    pids = []

    for path in SESSIONS.glob("*.json"):
        try:
            record = json.loads(path.read_text())
        except (OSError, ValueError):
            continue

        if str(record.get("sessionId", "")).startswith(session) and record.get("pid"):
            pids.append(int(record["pid"]))

    return pids


def run_applescript(script):
    result = subprocess.run(["osascript", "-e", script], capture_output=True, text=True, timeout=5)

    return result.stdout.strip() if result.returncode == 0 else f"error: {result.stderr.strip()}"


def target_clause(session):
    """AppleScript lines that set `target` to the session's terminal, or return "miss"."""
    pids = claude_pids(session)

    if len(pids) != 1:
        return None

    return f"""
        set matches to (every terminal whose pid is {pids[0]})
        if (count of matches) is not 1 then return "miss"
        set target to item 1 of matches"""


def send_keys(session, strokes):
    clause = target_clause(session)
    words = strokes.split(" ")
    parsed = [match for match in map(STROKE.match, words) if match]

    if clause is None or len(parsed) != len(words):
        return "miss"

    sends = "\n".join(
        f'        send key "{match[2]}" modifiers "{match[1]}" to target' if match[1]
        else f'        send key "{match[2]}" to target'
        for match in parsed
    )

    return run_applescript(f'tell application "Ghostty"{clause}\n{sends}\n        return "ok"\n    end tell')


def jump(session):
    clause = target_clause(session)

    if clause is None:
        return "miss"

    return run_applescript(f"""tell application "Ghostty"{clause}
        repeat with candidateWindow in windows
            repeat with candidateTab in tabs of candidateWindow
                if (id of focused terminal of candidateTab) is (id of target) then select tab candidateTab
            end repeat
        end repeat
        focus target
        activate
        return "ok"
    end tell""")


def open_port():
    fd = os.open(PORT, os.O_RDWR | os.O_NOCTTY)
    attributes = termios.tcgetattr(fd)
    attributes[3] &= ~(termios.ICANON | termios.ECHO)
    termios.tcsetattr(fd, termios.TCSANOW, attributes)

    return fd


def serve(fd):
    buffer = b""
    next_heartbeat = 0.0

    while True:
        if time.monotonic() >= next_heartbeat:
            os.write(fd, b"helper\n")
            next_heartbeat = time.monotonic() + HEARTBEAT_SECONDS

        if not select.select([fd], [], [], 0.5)[0]:
            continue

        data = os.read(fd, 1024)

        if not data:
            return

        buffer += data

        while b"\n" in buffer:
            line, buffer = buffer.split(b"\n", 1)
            fields = line.decode(errors="replace").strip().split("\t")

            match fields:
                case ["keys", session, strokes]:
                    log(f"keys {session} {strokes}: {send_keys(session, strokes)}")
                case ["jump", session]:
                    log(f"jump {session}: {jump(session)}")


def main():
    while True:
        if not os.path.exists(PORT):
            time.sleep(HEARTBEAT_SECONDS)
            continue

        try:
            fd = open_port()
        except OSError as error:
            log(f"cannot open {PORT}: {error}")
            time.sleep(HEARTBEAT_SECONDS)
            continue

        try:
            log(f"connected to {PORT}")
            serve(fd)
        except OSError as error:
            log(f"port closed: {error}")
        finally:
            os.close(fd)

        time.sleep(HEARTBEAT_SECONDS)


if __name__ == "__main__":
    sys.exit(main())
