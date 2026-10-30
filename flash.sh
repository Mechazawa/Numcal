#!/bin/bash
# Auto-flash script for RP2040
# Automatically reboots device into BOOTSEL mode before flashing

set -e

# Check if binary path is provided
if [ -z "$1" ]; then
    echo "Usage: $0 <binary.elf>"
    exit 1
fi

BINARY="$1"

# Try to reboot into BOOTSEL mode (ignore errors if already in BOOTSEL)
echo "Attempting to reboot device into BOOTSEL mode..."
picotool reboot -u -f 2>/dev/null || true

# The firmware also reboots into BOOTSEL on a "bootsel" line over its USB serial port
PORT=$(ls /dev/cu.usbmodem12345678* 2>/dev/null | head -n 1)
if [ -n "$PORT" ]; then
    # The port refuses to open while the device is still enumerating
    for _ in 1 2 3 4 5; do
        printf 'bootsel\n' 2>/dev/null > "$PORT" && break
        sleep 1
    done
    sleep 1.5
fi

# Give it a moment to reboot
sleep 0.5

# Now flash the binary
echo "Flashing $BINARY..."
picotool load --force --update --verify --execute -t elf "$BINARY"

echo "Flash complete!"
