#!/usr/bin/env python3
"""Delivers NumCal key presses to the Ghostty terminal of the Claude Code session shown on the device.

The device sends `keys\t<session>\t<strokes>` and `jump\t<session>` lines on its Claude serial port.
A session is matched to a terminal through the terminal's foreground pid and the session id Claude Code
records for that pid in ~/.claude/sessions/<pid>.json, so a reused tty or folder never selects the
wrong terminal. Without a match nothing is sent. Runs as a launchd agent, see nl.mechazawa.numcal-helper.plist.

The helper also relays the page 4 canvas for the `numcal` CLI over a unix socket. A client sends
`send <line>` to forward a line to the device, or `wait-key <seconds>` to receive the next
`key\t<name>` line from the device as `key <name>` (or `timeout`). `dump` asks the device for its screen
and replies `frame <hex>` (or `timeout`). Without a device, requests are answered with `error no device`.
"""

import json
import os
import re
import select
import socket
import subprocess
import sys
import termios
import time
from pathlib import Path

PORT = os.environ.get("NUMCAL_PORT", "/dev/cu.usbmodem123456783")
SOCKET = Path(os.environ.get("NUMCAL_SOCKET", Path.home() / ".cache" / "numcal" / "helper.sock"))
SESSIONS = Path(os.environ.get("NUMCAL_SESSIONS_DIR", Path.home() / ".claude" / "sessions"))
HEARTBEAT_SECONDS = 2
DUMP_TIMEOUT_SECONDS = 2
FRAME_HEX_LENGTH = 1024
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


class Relay:
    """Unix socket server that forwards canvas lines to the device and hands device keys to waiting clients."""

    def __init__(self, path):
        path.parent.mkdir(parents=True, exist_ok=True)
        path.unlink(missing_ok=True)
        self.listener = socket.socket(socket.AF_UNIX)
        self.listener.bind(str(path))
        path.chmod(0o600)
        self.listener.listen()
        self.buffers = {}
        self.deadlines = {}
        self.dump_deadlines = {}

    def sockets(self):
        return [self.listener, *self.buffers]

    def reply(self, client, text):
        try:
            client.sendall(f"{text}\n".encode())
        except OSError:
            self.drop(client)

    def drop(self, client):
        self.buffers.pop(client, None)
        self.deadlines.pop(client, None)
        self.dump_deadlines.pop(client, None)
        client.close()

    def handle(self, ready, port_fd):
        if ready is self.listener:
            client, _ = self.listener.accept()
            self.buffers[client] = b""

            return

        if ready not in self.buffers:
            return

        try:
            data = ready.recv(4096)
        except OSError:
            data = b""

        if not data:
            return self.drop(ready)

        self.buffers[ready] += data

        while ready in self.buffers and b"\n" in self.buffers[ready]:
            line, self.buffers[ready] = self.buffers[ready].split(b"\n", 1)
            self.request(ready, line.decode(errors="replace").rstrip("\r"), port_fd)

    def request(self, client, line, port_fd):
        if port_fd is None and line.split(" ", 1)[0] in ("send", "dump"):
            return self.reply(client, "error no device")

        match line.split(" ", 1):
            case ["send", text] if text.startswith("canvas\t"):
                os.write(port_fd, f"{text}\n".encode())
                self.reply(client, "ok")
            case ["wait-key", seconds] if re.fullmatch(r"\d+(\.\d+)?", seconds):
                self.deadlines[client] = time.monotonic() + float(seconds)
            case ["dump"]:
                if not self.dump_deadlines:
                    os.write(port_fd, b"dump\n")

                self.dump_deadlines[client] = time.monotonic() + DUMP_TIMEOUT_SECONDS
            case _:
                self.reply(client, "error bad request")

    def deliver_key(self, name):
        for client in list(self.deadlines):
            del self.deadlines[client]
            self.reply(client, f"key {name}")

    def deliver_frame(self, hex_data):
        for client in list(self.dump_deadlines):
            del self.dump_deadlines[client]
            self.reply(client, f"frame {hex_data}")

    def expire(self):
        now = time.monotonic()

        for waiting in (self.deadlines, self.dump_deadlines):
            for client in [client for client, deadline in waiting.items() if deadline <= now]:
                del waiting[client]
                self.reply(client, "timeout")


def read_device(fd, buffer, relay):
    """Handles the lines in the data the device sent. Returns the unfinished tail, or None when the port closed."""
    data = os.read(fd, 1024)

    if not data:
        return None

    *lines, tail = (buffer + data).split(b"\n")

    for line in lines:
        match line.decode(errors="replace").strip().split("\t"):
            case ["keys", session, strokes]:
                log(f"keys {session} {strokes}: {send_keys(session, strokes)}")
            case ["jump", session]:
                log(f"jump {session}: {jump(session)}")
            case ["key", name]:
                relay.deliver_key(name)
            case ["frame", hex_data] if re.fullmatch(f"[0-9a-fA-F]{{{FRAME_HEX_LENGTH}}}", hex_data):
                relay.deliver_frame(hex_data)

    return tail


def serve(fd, relay):
    buffer = b""
    next_heartbeat = 0.0

    while True:
        if time.monotonic() >= next_heartbeat:
            os.write(fd, b"helper\n")
            next_heartbeat = time.monotonic() + HEARTBEAT_SECONDS

        relay.expire()

        for ready in select.select([fd, *relay.sockets()], [], [], 0.5)[0]:
            if ready != fd:
                relay.handle(ready, fd)
            elif (buffer := read_device(fd, buffer, relay)) is None:
                return


def serve_without_device(relay, seconds):
    end = time.monotonic() + seconds

    while (remaining := end - time.monotonic()) > 0:
        relay.expire()

        for ready in select.select(relay.sockets(), [], [], min(remaining, 0.5))[0]:
            relay.handle(ready, None)


def main():
    relay = Relay(SOCKET)

    while True:
        if not os.path.exists(PORT):
            serve_without_device(relay, HEARTBEAT_SECONDS)
            continue

        try:
            fd = open_port()
        except OSError as error:
            log(f"cannot open {PORT}: {error}")
            serve_without_device(relay, HEARTBEAT_SECONDS)
            continue

        try:
            log(f"connected to {PORT}")
            serve(fd, relay)
        except OSError as error:
            log(f"port closed: {error}")
        finally:
            os.close(fd)

        serve_without_device(relay, HEARTBEAT_SECONDS)


if __name__ == "__main__":
    sys.exit(main())
