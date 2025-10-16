# Quick Start Guide

This guide will help you get up and running with the NumCal keyboard firmware quickly.

## One-Time Setup

1. **Install Rust toolchain:**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

2. **Add ARM target:**
```bash
rustup target add thumbv6m-none-eabi
```

3. **Install required tools:**
```bash
# Essential
cargo install flip-link

# For flashing (choose one)
brew install picotool                    # macOS (recommended)
cargo install elf2uf2-rs                 # Alternative (manual drag-drop)
cargo install probe-rs --features cli   # If you have a debug probe
```

## Build and Flash

### Quick Flash with picotool (Recommended)

```bash
# Put RP2040 in bootloader mode:
# 1. Hold BOOTSEL button
# 2. Connect USB (or press RESET while holding BOOTSEL)
# 3. Release BOOTSEL

# Build and flash in one command
cargo run --release
```

### Alternative: Manual UF2 Flash

```bash
# Build release version
cargo build --release

# Convert to UF2
elf2uf2-rs target/thumbv6m-none-eabi/release/numcal numcal.uf2

# Put RP2040 in bootloader mode (same as above)
# Drag and drop numcal.uf2 onto the RPI-RP2 drive
```

### Development Build

```bash
cargo build
elf2uf2-rs target/thumbv6m-none-eabi/debug/numcal numcal.uf2
# Flash as above
```

## Pin Connections

### Keyboard Matrix
- **Rows (outputs):** GP9, GP8, GP7, GP6, GP5, GP4
- **Columns (inputs):** GP26, GP27, GP28, GP29

### OLED Display (SPI)
- **SCK:** GP14
- **MOSI:** GP15
- **CS:** GP10
- **DC:** GP13
- **RST:** GP3
- **VCC:** 3.3V
- **GND:** GND

### USB
Connect your RP2040 board's USB port to your computer.

## Testing

After flashing:

1. The OLED display should show "Hello World"
2. Connect USB to your computer
3. The device should appear as "NumCal Keyboard"
4. Press keys on your matrix - they should register as keypresses

## Troubleshooting

**Issue:** Compilation fails
- Solution: Make sure `thumbv6m-none-eabi` target is installed: `rustup target add thumbv6m-none-eabi`

**Issue:** UF2 conversion fails
- Solution: Install elf2uf2-rs: `cargo install elf2uf2-rs`

**Issue:** Display doesn't work
- Solution: Check all SPI connections, ensure 3.3V power

**Issue:** Keys don't register
- Solution: Verify matrix wiring matches pin configuration

**Issue:** USB device not recognized
- Solution: Try a different USB cable (must support data, not just power)

## Next Steps

- Customize the keymap in `src/main.rs` (search for `KEYMAP`)
- Modify display content in the `display_task()` function
- Add layers or macros to enhance functionality
- Implement key press indicators on the display

## Learning Resources

- [Embassy Documentation](https://embassy.dev)
- [RP2040 Datasheet](https://datasheets.raspberrypi.com/rp2040/rp2040-datasheet.pdf)
- [USB HID Usage Tables](https://www.usb.org/hid)
