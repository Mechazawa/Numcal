#!/bin/bash
# USB Serial Monitor for NumCal
# Usage: ./monitor-serial.sh [duration_in_seconds]

DURATION=${1:-5}

# Auto-detect USB serial port
SERIAL_PORT=$(ls /dev/cu.usbmodem* 2>/dev/null | head -n 1)

if [ -z "$SERIAL_PORT" ]; then
    echo "Error: No USB serial device found (looking for /dev/cu.usbmodem*)"
    echo "Available serial devices:"
    ls /dev/cu.* 2>/dev/null | grep -v Bluetooth
    exit 1
fi

echo "Found device: $SERIAL_PORT"
echo "Monitoring for $DURATION seconds..."
echo "Press Ctrl+C to stop early"
echo "----------------------------------------"

# Use gtimeout if available (brew install coreutils), otherwise use perl
{
    if command -v gtimeout &> /dev/null; then
        gtimeout "$DURATION" cat "$SERIAL_PORT" 2>/dev/null
        EXIT_CODE=$?
    else
        # Fallback using perl with alarm
        perl -e "alarm $DURATION; exec 'cat', '$SERIAL_PORT'" 2>/dev/null
        EXIT_CODE=$?
    fi
} 2>/dev/null

echo ""
echo "----------------------------------------"
if [ $EXIT_CODE -eq 124 ] || [ $EXIT_CODE -eq 142 ] || [ $EXIT_CODE -eq 0 ]; then
    echo "Monitoring completed"
else
    echo "Monitoring finished (exit code: $EXIT_CODE)"
fi
