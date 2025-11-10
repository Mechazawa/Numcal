# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

NumCal is an RP2040-based custom keyboard firmware featuring a 6x4 matrix keypad with OLED display and USB HID support. Built using Embassy async runtime for embedded Rust.

## Build Commands

**Target**: `thumbv6m-none-eabi` (Cortex-M0+ ARM)

```bash
# Build release
cargo build --release

# Build debug
cargo build

# Build and flash (requires picotool)
cargo run --release

# Clippy
cargo clippy

# Memory/size analysis
cargo size --release
cargo bloat --release
cargo nm --release
```

## Flashing

The project uses `flash.sh` as the cargo runner (see `.cargo/config.toml`). It automatically:
1. Reboots the device into BOOTSEL mode via `picotool reboot -u -f`
2. Flashes the ELF binary using `picotool load`

**Alternative manual flash**: Use `elf2uf2-rs` to convert ELF to UF2, then drag-drop onto the RPI-RP2 drive.

## Architecture

### Task System (Embassy Executor)

The firmware is organized into concurrent async tasks spawned by Embassy:

- **USB task** (`tasks/usb.rs`): Manages USB device, HID keyboard interface, and CDC-ACM logging
- **Display task** (`tasks/display.rs`): Renders to SSD1306 OLED via SPI using a channel-based command queue
- **Keypad task** (`tasks/keypad.rs`): Scans 6x4 matrix, debounces keys, publishes events to `KEYPAD_CHANNEL`
- **Hotkeys task** (`tasks/hotkeys.rs`): Listens for F1+F2+F3+F4 combo to reboot into BOOTSEL mode
- **Mode handler** (`modes/mod.rs`): Runs the active mode's task loop and handles mode switching

### Communication Channels

- `KEYPAD_CHANNEL` (PubSub): Broadcasts `KeyEvent` from keypad scanner to multiple subscribers
- `HID_CHANNEL` (MPSC): Sends `HidEvent` commands to USB HID task
- `DISPLAY_CHANNEL` (MPSC): Queues `DisplayAction` commands for rendering

### Mode System

Modes implement the `Mode` trait with an async `task()` method. The mode handler:
- Runs the current mode's task in a loop
- Monitors `TARGET_MODE` and `MODE_RUNNING` atomics
- Switches modes when Lock+F1/F2/F3/F4 is pressed (handled by `mode_switcher_task`)
- Uses `enum_dispatch` for zero-cost dispatch to mode implementations

Current modes:
- `BootMode`: Displays splash screen for 2 seconds on startup
- `NumpadMode`: Standard numpad with HID passthrough (Lock+Fn excluded)

### Hardware Configuration

**Keypad Matrix**:
- Rows (outputs): GP9, GP8, GP7, GP6, GP5, GP4
- Cols (inputs with pull-up): GP26, GP27, GP28, GP29

**OLED Display (SSD1306, SPI)**:
- SCK: GP14, MOSI: GP15, CS: GP10, DC: GP13, RST: GP3

**Keymap** (6 rows × 4 cols):
```
Row 0: F1    F2   F3   F4
Row 1: Lock  Div  Mul  Sub
Row 2: 7     8    9    NC
Row 3: 4     5    6    Add
Row 4: 1     2    3    NC
Row 5: NC    0    Dot  Enter
```

### DisplayProxy Pattern

`DisplayProxy` is a channel-backed draw target that implements `embedded_graphics::DrawTarget`. It sends drawing commands to the display task instead of blocking on SPI operations. Always call `flush()` after drawing to send the framebuffer to the display.

### Debouncing

Key state changes go through `Debounce<bool>` with 10ms delay. The debouncer tracks measured value and timestamp, only updating the stable value after delay expires.

## Key Implementation Details

- **Static cells**: Global resources use `StaticCell` for safe one-time initialization
- **Atomic state**: Keypad state stored in 2D `AtomicBool` array for lock-free `key_pressed()` queries
- **Inverse keymap**: `KEYMAP_INV` provides O(1) Key enum to (row, col) lookup for state queries
- **Memory layout**: `memory.x` defines RP2040 flash/RAM regions and boot2 section
- **Linker flags**: Uses `flip-link` for stack overflow protection, links `defmt.x` for logging
- **Profile**: Release uses LTO, opt-level "z" (size), codegen-units 1 for minimal binary size

## Adding a New Mode

1. Create `src/modes/{name}.rs` with a struct implementing `Mode` trait
2. Add variant to `CurrentMode` enum in `modes/mod.rs`
3. Update `mode_handler_task()` match to instantiate your mode
4. Assign an F-key (F1-F4) in `mode_switcher_task()` match

## Common Patterns

**Publishing key events**:
```rust
KEYPAD_CHANNEL.publisher().unwrap().publish_immediate(event);
```

**Subscribing to key events**:
```rust
let mut receiver = KEYPAD_CHANNEL.subscriber().unwrap();
if let WaitResult::Message(event) = receiver.next_message().await { ... }
```

**Sending HID events**:
```rust
HID_CHANNEL.sender().send(HidEvent::SetKey(keycode)).await;
```

**Drawing to display**:
```rust
let mut display = DisplayProxy::new();
display.clear(BinaryColor::Off).unwrap();
Text::new("Hello", Point::new(5, 38), text_style).draw(&mut display).unwrap();
display.flush().unwrap();
```
