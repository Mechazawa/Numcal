#!/usr/bin/env python3
"""Talk to firmware built with `--features debug-link`.

    numcal-debug.py key 1 Sub Enter                 tap keys (names as in the Key enum, digits as 0-9)
    numcal-debug.py down Lock / up Lock             hold or release a key
    numcal-debug.py send "<line>"                   send a raw line, such as a fake hook status

To see the screen, use `numcal dump [--png out.png]` from claude/numcal.
"""

import argparse
import os
import sys
import termios
import time

PORT = "/dev/cu.usbmodem123456781"


def open_port():
    if not os.path.exists(PORT):
        sys.exit(f"NumCal log port {PORT} not found")

    fd = os.open(PORT, os.O_RDWR | os.O_NOCTTY | os.O_NONBLOCK)
    attributes = termios.tcgetattr(fd)
    attributes[3] &= ~(termios.ICANON | termios.ECHO)
    termios.tcsetattr(fd, termios.TCSANOW, attributes)

    return fd


def send(fd, line):
    os.write(fd, (line + "\n").encode())
    termios.tcdrain(fd)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("command", choices=["key", "down", "up", "send"])
    parser.add_argument("args", nargs="*")
    options = parser.parse_args()

    fd = open_port()

    if options.command == "send":
        send(fd, " ".join(options.args))
    elif options.command == "key":
        for key in options.args:
            send(fd, f"key {key}")
            time.sleep(0.15)
    else:
        send(fd, f"{options.command} {options.args[0]}")

    os.close(fd)


if __name__ == "__main__":
    main()
