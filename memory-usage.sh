#!/bin/bash
# Memory usage analysis script for NumCal firmware

set -e

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
BOLD='\033[1m'
NC='\033[0m' # No Color

# Memory limits from memory.x
FLASH_SIZE=$((2048 * 1024 - 256))  # 2MB - 256 bytes for BOOT2
RAM_SIZE=$((264 * 1024))            # 264KB

# Convert bytes to human readable
human_readable() {
    local bytes=$1
    if [ $bytes -lt 1024 ]; then
        echo "${bytes}B"
    elif [ $bytes -lt $((1024 * 1024)) ]; then
        echo "$(awk "BEGIN {printf \"%.2f\", $bytes/1024}")KB"
    else
        echo "$(awk "BEGIN {printf \"%.2f\", $bytes/(1024*1024)}")MB"
    fi
}

# Get color based on percentage
get_color() {
    local percent=$1
    if [ $percent -lt 50 ]; then
        echo "$GREEN"
    elif [ $percent -lt 80 ]; then
        echo "$YELLOW"
    else
        echo "$RED"
    fi
}

# Draw progress bar
progress_bar() {
    local percent=$1
    local width=40
    local filled=$((percent * width / 100))
    local empty=$((width - filled))

    printf "["
    printf "%${filled}s" | tr ' ' '#'
    printf "%${empty}s" | tr ' ' '-'
    printf "]"
}

echo -e "${BOLD}=== NumCal Memory Usage ===${NC}\n"

# Build and get size info
echo "Building release binary..."
cargo build --release 2>&1 | grep -E "(Compiling|Finished)" | tail -1

# Get section sizes
SIZE_OUTPUT=$(cargo size --release -- -A 2>&1)

# Extract section sizes
TEXT=$(echo "$SIZE_OUTPUT" | grep "^\.text" | awk '{print $2}')
RODATA=$(echo "$SIZE_OUTPUT" | grep "^\.rodata" | awk '{print $2}')
BOOT2=$(echo "$SIZE_OUTPUT" | grep "^\.boot2" | awk '{print $2}')
DATA=$(echo "$SIZE_OUTPUT" | grep "^\.data" | awk '{print $2}')
BSS=$(echo "$SIZE_OUTPUT" | grep "^\.bss" | awk '{print $2}')
UNINIT=$(echo "$SIZE_OUTPUT" | grep "^\.uninit" | awk '{print $2}')

# Calculate totals
FLASH_USED=$((TEXT + RODATA + BOOT2))
RAM_USED=$((DATA + BSS + UNINIT))

# Calculate percentages
FLASH_PERCENT=$(awk "BEGIN {printf \"%.1f\", ($FLASH_USED * 100.0) / $FLASH_SIZE}")
RAM_PERCENT=$(awk "BEGIN {printf \"%.1f\", ($RAM_USED * 100.0) / $RAM_SIZE}")

# Get colors
FLASH_COLOR=$(get_color ${FLASH_PERCENT%.*})
RAM_COLOR=$(get_color ${RAM_PERCENT%.*})

echo ""
echo -e "${BOLD}${BLUE}FLASH Memory:${NC}"
echo -e "  Total:     $(human_readable $FLASH_SIZE)"
echo -e "  Used:      ${FLASH_COLOR}$(human_readable $FLASH_USED)${NC}"
echo -e "  Free:      $(human_readable $((FLASH_SIZE - FLASH_USED)))"
echo -e "  Usage:     ${FLASH_COLOR}${FLASH_PERCENT}%${NC}"
echo -n "  "
progress_bar ${FLASH_PERCENT%.*}
echo ""
echo -e "  Breakdown:"
echo -e "    .text   (code):       $(human_readable $TEXT)"
echo -e "    .rodata (constants):  $(human_readable $RODATA)"
echo -e "    .boot2  (bootloader): $(human_readable $BOOT2)"

echo ""
echo -e "${BOLD}${BLUE}RAM Memory:${NC}"
echo -e "  Total:     $(human_readable $RAM_SIZE)"
echo -e "  Used:      ${RAM_COLOR}$(human_readable $RAM_USED)${NC}"
echo -e "  Free:      $(human_readable $((RAM_SIZE - RAM_USED)))"
echo -e "  Usage:     ${RAM_COLOR}${RAM_PERCENT}%${NC}"
echo -n "  "
progress_bar ${RAM_PERCENT%.*}
echo ""
echo -e "  Breakdown:"
echo -e "    .data   (initialized): $(human_readable $DATA)"
echo -e "    .bss    (zero-init):   $(human_readable $BSS)"
echo -e "    .uninit (uninit):      $(human_readable $UNINIT)"

# Check if cargo-bloat is installed
if command -v cargo-bloat &> /dev/null; then
    echo ""
    echo -e "${BOLD}${BLUE}Top 10 Flash Space Users:${NC}"
    cargo bloat --release -n 10 2>/dev/null | tail -n +2

    echo ""
    echo -e "${BOLD}${BLUE}Top 10 RAM Users (Data):${NC}"
    cargo bloat --release -n 10 --data 2>/dev/null | tail -n +2
else
    echo ""
    echo -e "${YELLOW}Tip: Install cargo-bloat for detailed size analysis:${NC}"
    echo "  cargo install cargo-bloat"
fi

echo ""
