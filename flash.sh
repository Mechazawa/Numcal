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

# Give it a moment to reboot
sleep 0.5

# Now flash the binary
echo "Flashing $BINARY..."
picotool load --force --update --verify --execute -t elf "$BINARY"

echo "Flash complete!"
