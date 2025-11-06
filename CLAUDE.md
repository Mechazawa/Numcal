# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

NumCal is an RP2040-based custom keyboard firmware using the Embassy async framework. It features a 4x6 keyboard matrix, SSD1305 OLED display (128x32), and USB HID keyboard support.

## Essential Build Commands

### Building
```bash
# Release build (optimized for size with "z" optimization)
cargo build --release

# Development build
cargo build
```

### Flashing to Hardware

**Method 1: picotool (Recommended - configured as default runner)**
```bash
# Put RP2040 in bootloader mode first (hold BOOTSEL, connect USB, release)
cargo run --release
```

**Method 2: UF2 manual flash**
```bash
cargo build --release
elf2uf2-rs target/thumbv6m-none-eabi/release/numcal numcal.uf2
# Drag-drop numcal.uf2 to RPI-RP2 drive
```

**Method 3: With debug probe**
```bash
# Change runner in .cargo/config.toml to: probe-rs run --chip RP2040
cargo run --release
```

## Critical Build Configuration

### Embassy Framework Requirements
- **MUST** include `critical-section-impl` feature in embassy-rp - Cortex-M0+ lacks native atomics
- Boot2 bootloader is statically embedded via `BOOT2` constant in main.rs using `rp2040-boot2` crate
- `memory.x` defines BOOT2, FLASH, and RAM regions - do NOT duplicate linker args in build.rs

### Dependency Compatibility
- ssd1306 version 0.10+ required for embedded-hal 1.0 compatibility
- SPI devices must be wrapped with `ExclusiveDevice` from `embedded-hal-bus` before use with display drivers
- `portable-atomic` with `critical-section` feature required for Cortex-M0+ atomic operations

### Linker Configuration
- Linker args (-Tlink.x, -Tdefmt.x) are in `.cargo/config.toml` ONLY - do NOT duplicate in build.rs
- Uses `flip-link` for stack overflow protection
- Build target is `thumbv6m-none-eabi` (Cortex-M0+)

## Hardware Pin Assignments

### Critical SPI1 Pins (validated for RP2040)
- CLK: GP14 (not GP15 - that's MOSI)
- MOSI: GP15 (not GP16 - invalid for SPI1)
- CS: GP10
- DC: GP13
- RST: GP3

### Keyboard Matrix
- Rows (outputs, active-low): GP9, GP8, GP7, GP6, GP5, GP4
- Columns (inputs, pull-up): GP26, GP27, GP28, GP29

To change pins, modify spawner calls in `main()` function (src/main.rs:80-89).

## Architecture

### Async Task Structure
The firmware uses Embassy's cooperative multitasking with independent tasks:

1. **keyboard_task** (src/keyboard.rs) - Matrix scanner with debouncing
   - Scans 4x6 matrix by driving rows LOW sequentially
   - 10ms software debounce (configurable via `DEBOUNCE_MS` constant)
   - Sends key events to USB and display tasks via channels

2. **display_task** (src/display.rs) - OLED management
   - Renders text to SSD1305 OLED via SPI at 8MHz
   - Uses ssd1306 driver with ExclusiveDevice wrapper
   - Receives display updates via channel

3. **usb_device_task** (src/usb.rs) - USB device manager
   - Runs the USB device stack

4. **usb_hid_task** (src/usb.rs) - USB HID keyboard
   - Creates USB HID keyboard interface (VID: 0x16c0, PID: 0x27dd)
   - Sends HID keyboard reports based on key events

5. **logger_task** (src/main.rs) - USB serial logging
   - Provides USB CDC-ACM serial port for logging
   - Outputs `log` crate messages to serial console

Tasks are spawned in main() and run concurrently. The main task keeps the executor alive with a 60-second sleep loop.

### Keymap Configuration
The `KEYMAP` constant (src/main.rs:45-52) maps matrix positions [row][col] to USB HID keycodes. It's a 2D array indexed by row then column. Use 0x00 for unused keys.

Example: `KEYMAP[0][0] = 0x27` maps row 0, col 0 to HID keycode 0x27 (numpad 0).

Reference: https://www.usb.org/sites/default/files/documents/hut1_12v2.pdf

## Common Development Tasks

### Customizing the Keymap
Edit the `KEYMAP` constant in src/main.rs:45-52. Each byte is a USB HID keycode.

### Adjusting Debounce Time
Modify `DEBOUNCE_MS` constant in src/main.rs:41 (default: 10ms).

### Viewing Logs
Logs use the `log` crate with USB serial transport via `embassy-usb-logger`. No debug probe required!

**Viewing logs:**
1. Flash the firmware: `cargo run --release`
2. Device enumerates as both a USB HID keyboard and USB serial port
3. Connect to serial port (typically `/dev/tty.usbmodem*` on macOS):
   ```bash
   screen /dev/tty.usbmodem* 115200
   # or
   minicom -D /dev/tty.usbmodem*
   ```

Logs use standard `log` macros: `log::info!()`, `log::error!()`, `log::debug!()`, etc.
Log level is configured in `logger_task()` (default: `Info`).

### Binary Size
Target size: ~1.2MB (fits in 2MB flash). Release profile uses:
- `opt-level = "z"` - optimize for size
- `lto = true` - link-time optimization
- `codegen-units = 1` - single codegen unit

## Important Implementation Notes

### Embassy Boot2 Integration
- The RP2040 requires a 256-byte boot2 bootloader in the first flash sector
- This is provided via `pub static BOOT2: [u8; 256] = rp2040_boot2::BOOT_LOADER_W25Q080`
- W25Q080 variant matches most RP2040 boards' flash chips
- The linker script places this in the BOOT2 memory region

### SPI Device Wrapping Pattern
Embassy-rp's SPI implements embedded-hal 1.0's `SpiBus` trait but NOT `SpiDevice`. Display drivers need `SpiDevice`, so wrap with:
```rust
let spi_device = ExclusiveDevice::new_no_delay(spi, cs_pin).unwrap();
```

### Cortex-M0+ Limitations
- No native atomic CAS operations
- Requires `critical-section-impl` feature in embassy-rp
- Requires `portable-atomic` with `critical-section` feature
- Cannot use probe-rs for flashing by default (picotool is preferred)

## Known Issues and Gotchas

- The firmware uses USB composite device (HID + CDC-ACM). Some older USB hosts may have issues with composite devices.
- USB serial logs only appear after the USB device is fully enumerated (takes ~1-2 seconds after reset).
- Early boot logs (before USB is initialized) are not visible. Consider keeping `defmt` for probe-based debugging if needed.

## Project Structure

```
src/main.rs          - Main entry point, USB setup, logger task
src/keyboard.rs      - Keyboard matrix scanning and state management
src/display.rs       - OLED display rendering task
src/usb.rs           - USB HID keyboard tasks
src/modes/           - Keyboard modes (numpad, calculator, etc.)
Cargo.toml           - Dependencies with embassy-rp features
memory.x             - RP2040 memory layout (BOOT2, FLASH, RAM)
build.rs             - Copies memory.x to build output
.cargo/config.toml   - Target config, runner (picotool), rustflags
```

## Logging Implementation

The firmware uses `embassy-usb-logger` to provide USB serial logging without requiring a debug probe:

- **Logger crate**: Uses standard `log` crate macros (`log::info!()`, `log::error!()`, etc.)
- **USB CDC-ACM**: Creates a second USB interface (serial port) alongside the HID keyboard
- **Composite device**: Both HID keyboard and serial port work simultaneously
- **Buffer size**: 1024 bytes (configurable in `logger_task()`)
- **Log level**: Default is `Info` (configurable in `logger_task()`)
