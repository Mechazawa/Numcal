# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

NumCal is an embedded Rust firmware for a custom numpad/calculator device running on the RP2040 microcontroller (Raspberry Pi Pico). It uses Embassy (async embedded framework) to manage concurrent tasks for USB HID, keyboard matrix scanning, OLED display updates, and hotkey detection.

**Key Technologies:**
- **Target:** `thumbv6m-none-eabi` (ARM Cortex-M0+)
- **Runtime:** Embassy async executor with cooperative multitasking
- **Display:** SSD1306 128x64 OLED (SPI interface)
- **USB:** CDC-ACM for logging + USB HID (planned)
- **Build:** Optimized for size (`opt-level = "z"`, LTO enabled)

## Essential Commands

### Building & Flashing

```bash
# Build release version (optimized for size)
cargo build --release

# Build and flash automatically (requires picotool)
cargo run --release

# Development build
cargo build

# Analyze memory usage
./memory-usage.sh          # Basic report
./memory-usage.sh -v       # Verbose with top consumers
```

### Size Analysis Tools

```bash
# Section sizes
cargo size --release -- -A

# Top code size contributors
cargo bloat --release -n 10

# RAM usage analysis
cargo bloat --release -n 10 --data

# Symbol sizes
rust-nm --print-size --size-sort --radix=d target/thumbv6m-none-eabi/release/numcal
```

### Flashing Methods

1. **Automatic (recommended):** `cargo run --release` uses `flash.sh` which auto-reboots device via picotool
2. **Manual UF2:** Build, convert with `elf2uf2-rs`, drag-drop to RPI-RP2 drive
3. **Debug probe:** Use `probe-rs` if hardware debugger is connected

## Architecture

### Task-Based Concurrency

The firmware uses Embassy's executor to run multiple cooperative tasks simultaneously:

1. **USB Device Task** (`tasks/usb.rs`) - Handles USB enumeration and CDC-ACM logging
2. **Display Task** (`tasks/display.rs`) - Asynchronous display rendering via channel
3. **Keypad Task** (`tasks/keypad.rs`) - Matrix scanning with debouncing, publishes key events
4. **Hotkey Task** (`tasks/hotkeys.rs`) - Monitors F1+F2+F3+F4 combo for BOOTSEL reboot

Tasks communicate via:
- **Channels** (`embassy_sync::channel`) for display commands
- **PubSub** (`embassy_sync::pubsub`) for keyboard events

### Display Architecture

The display uses a proxy pattern to allow any task to draw without blocking:

- `DisplayProxy` implements `DrawTarget` from `embedded-graphics`
- Drawing commands are buffered into `DisplayAction` enums and sent via channel
- The dedicated display task processes commands and calls the SSD1306 driver
- This prevents SPI bus contention and allows non-blocking graphics updates

### Keyboard Matrix Scanning

6 rows × 4 columns = 24 keys total:
- **Rows** (GP4-GP9): Output pins, driven low sequentially
- **Columns** (GP26-GP29): Input pins with pull-ups

Scanning process:
1. Set one row low, others high
2. Read all column pins (low = pressed)
3. Debounce using `Debounce<bool>` utility (10ms threshold)
4. Publish `KeyEvent` to PubSub channel on state changes
5. Store state in atomic bool array for synchronous `key_pressed()` queries

### Memory Constraints

- **Flash:** 2MB - 256 bytes (BOOT2 bootloader)
- **RAM:** 264KB
- **Build profile:** Optimized for size with full LTO
- Use `heapless::Vec` instead of `std::Vec` (no heap allocator)
- Static allocation via `static_cell::StaticCell` for task-owned resources

### Pin Configuration

**Keyboard Matrix:**
- Rows: GP9, GP8, GP7, GP6, GP5, GP4
- Columns: GP26, GP27, GP28, GP29

**OLED (SPI1):**
- SCK: GP14, MOSI: GP15, CS: GP10
- DC: GP13, RST: GP3

## Development Guidelines

### Adding New Tasks

1. Create module in `src/tasks/`
2. Define task function with `#[embassy_executor::task]` attribute
3. Add init function that spawns task via `Spawner`
4. Export from `tasks/mod.rs`
5. Call init from `main.rs` before entering main loop

### Memory Management

- Use `StaticCell` for 'static references needed by tasks
- Prefer `heapless` collections with compile-time size bounds
- Check size impact with `./memory-usage.sh -v` after changes
- Use `portable_atomic` for lock-free state sharing between tasks

### Display Updates

```rust
use crate::tasks::DisplayProxy;
use embedded_graphics::prelude::*;

let mut display = DisplayProxy::new();
display.clear(BinaryColor::Off).unwrap();
// ... draw operations ...
display.flush().unwrap();  // Commit changes to screen
```

### Keyboard Events

```rust
use crate::tasks::{KEYPAD_CHANNEL, KeyEvent, key_pressed, Key};

// Subscribe to events
let mut sub = KEYPAD_CHANNEL.subscriber().unwrap();
while let WaitResult::Message(event) = sub.next_message().await {
    // Handle event.key and event.pressed
}

// Query current state synchronously
if key_pressed(Key::F1) {
    // F1 is currently held down
}
```

### Logging

USB CDC-ACM logger is initialized automatically. Use standard `log` macros:

```rust
use log::{info, warn, error};
info!("Message");  // Sent over USB serial
```

Note: Logger requires ~2 seconds after boot to enumerate and become ready.

## Common Patterns

### Debouncing Inputs

```rust
use crate::utils::debounce::Debounce;
let mut debouncer = Debounce::new(false, Duration::from_millis(10));
if debouncer.measure(new_value) {
    // Value changed and stabilized
}
```

### Inter-Task Communication

- **One-to-one:** Use `Channel<Mutex, T, CAPACITY>`
- **Broadcast:** Use `PubSubChannel` with multiple subscribers
- **Shared state:** Use `portable_atomic` types with `Ordering::Relaxed`

### Rebooting to BOOTSEL Mode

```rust
use embassy_rp::rom_data::reset_to_usb_boot;
reset_to_usb_boot(0, 0);  // Reboot for flashing via UF2
```

## Troubleshooting

- **Linker errors:** Ensure `flip-link` is installed and `thumbv6m-none-eabi` target added
- **Memory overflow:** Check `./memory-usage.sh`, reduce buffer sizes in `heapless` collections
- **Display artifacts:** Ensure `flush()` is called after drawing operations
- **Keys not registering:** Verify debounce threshold and matrix wiring
- **USB not working:** Wait 2s after boot for enumeration, check cable supports data
