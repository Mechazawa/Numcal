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
- Boot2 bootloader is handled internally by embassy-rp - no manual declaration needed
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
The firmware uses Embassy's cooperative multitasking. Currently implemented:

1. **display_task** (src/tasks/display.rs) - Uptime counter display
   - Shows elapsed time since boot in hh:mm:ss format
   - Uses SSD1306 128x64 OLED via SPI1 at 8MHz
   - Uses ssd1306 driver with ExclusiveDevice wrapper
   - Updates every second using embassy_time::Timer
   - Uses embedded_graphics for text rendering

Tasks are spawned in main() and run concurrently. The main task keeps the executor alive with a 60-second sleep loop.

## Common Development Tasks

### Viewing Logs
Logs use `defmt` with RTT transport. With a debug probe:
```bash
cargo run --release  # Logs appear in terminal
```

Without a probe, logs are not visible (use picotool method for flashing only).

### Binary Size
Target size: ~1.2MB (fits in 2MB flash). Release profile uses:
- `opt-level = "z"` - optimize for size
- `lto = true` - link-time optimization
- `codegen-units = 1` - single codegen unit

## Important Implementation Notes

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

- Only display task is currently implemented - keyboard and USB tasks not yet added
- Reference implementation code is in `reference/` directory for future feature development

## Project Structure

```
src/
  main.rs                - Hardware initialization and task spawning
  tasks/
    mod.rs               - Task module declarations
    display.rs           - Display task with uptime counter
reference/               - Previous implementation for reference
Cargo.toml               - Dependencies with embassy-rp features
memory.x                 - RP2040 memory layout (BOOT2, FLASH, RAM)
build.rs                 - Copies memory.x to build output
.cargo/config.toml       - Target config, runner (picotool), rustflags
```
