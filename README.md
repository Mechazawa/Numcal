# NumCal - RP2040 Custom Keyboard Firmware

A custom keyboard firmware for RP2040-based keyboards using the Embassy async framework. This firmware supports a 4x6 key matrix and an SSD1305 OLED display (128x32).

## Features

- **Async architecture** using Embassy framework for efficient concurrent operations
- **4x6 keyboard matrix** scanning with configurable pin mapping
- **Hardware debouncing** (10ms default) for reliable key detection
- **USB HID keyboard** support for sending keypresses to the host
- **SSD1305 OLED display** support via SPI (128x32 resolution)
- **Embedded graphics** for drawing text and graphics on the display
- **Low power consumption** optimized for battery-powered operation

## Hardware Configuration

### Keyboard Matrix

- **Columns (4 pins):** GP26, GP27, GP28, GP29 - configured as inputs with pull-up resistors
- **Rows (6 pins):** GP9, GP8, GP7, GP6, GP5, GP4 - configured as outputs

The matrix scanning works by driving each row LOW sequentially and reading the column states. A LOW reading on a column indicates a key press at that row/column intersection.

### SSD1305 OLED Display (SPI)

- **SCK (Clock):** GP15
- **MOSI (Data):** GP16
- **CS (Chip Select):** GP10
- **DC (Data/Command):** GP14
- **Reset:** GP3

The display runs at 8 MHz SPI clock speed and shows "Hello World" on startup.

### USB

The keyboard appears as a standard USB HID keyboard device with the following identifiers:
- **VID:** 0x16c0 (Generic)
- **PID:** 0x27dd (Generic)
- **Manufacturer:** NumCal
- **Product:** NumCal Keyboard

## Prerequisites

Before building, you need to install the following tools:

### 1. Rust and Cargo

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### 2. ARM Cortex-M Target

```bash
rustup target add thumbv6m-none-eabi
```

### 3. Additional Tools

```bash
# flip-link for stack overflow protection
cargo install flip-link

# picotool for flashing via USB (recommended - no debug probe needed)
# On macOS: brew install picotool
# On Linux: See https://github.com/raspberrypi/picotool

# Alternative: elf2uf2-rs for creating UF2 files (manual drag-drop)
cargo install elf2uf2-rs

# Optional: probe-rs for development with a debug probe
cargo install probe-rs --features cli
```

## Building the Firmware

### Development Build

```bash
cargo build
```

### Release Build (Optimized)

```bash
cargo build --release
```

The compiled binary will be in `target/thumbv6m-none-eabi/release/numcal`.

## Flashing the Firmware

There are three methods to flash the firmware to your RP2040:

### Method 1: picotool (Recommended - No Debug Probe Needed)

This method uses picotool to flash directly via USB - the easiest and fastest option.

1. **Enter bootloader mode:**
   - Hold the BOOTSEL button on your RP2040 board
   - While holding BOOTSEL, connect the USB cable (or press RESET)
   - Release BOOTSEL

2. **Flash and run:**

```bash
# Build and flash in one command
cargo run --release

# Or build, flash, verify, and execute
cargo build --release
picotool load --update --verify --execute -t elf target/thumbv6m-none-eabi/release/numcal
```

The firmware will be automatically flashed and started on your board.

### Method 2: USB Bootloader (UF2) - Manual Drag-and-Drop

This method uses the RP2040's built-in USB bootloader.

1. **Convert ELF to UF2:**

```bash
# For release builds
cargo build --release
elf2uf2-rs target/thumbv6m-none-eabi/release/numcal numcal.uf2
```

2. **Enter bootloader mode:**
   - Hold the BOOTSEL button on your RP2040 board
   - While holding BOOTSEL, connect the USB cable (or press RESET)
   - Release BOOTSEL
   - The RP2040 will appear as a USB mass storage device named "RPI-RP2"

3. **Flash the firmware:**
   - Simply drag and drop the `numcal.uf2` file onto the RPI-RP2 drive
   - The board will automatically reboot and run the new firmware

### Method 3: probe-rs (For Development with Debug Probe)

If you have a debug probe (like a Raspberry Pi Pico configured as a probe):

```bash
# Change runner in .cargo/config.toml to: probe-rs run --chip RP2040
# Then:
cargo run --release
```

## Keymap

The default keymap is configured for a numpad-style layout:

```
Row 0: [0] [8] [9] [7]
Row 1: [1] [5] [6] [4]
Row 2: [2] [*] [3] [Q]
Row 3: [Enter] [Space] [Down] [Up]
Row 4: [Right] [Left] [Backspace] [Esc]
Row 5: [Tab] [CapsLock] [-] [-]
```

To customize the keymap, edit the `KEYMAP` constant in `src/main.rs`. The values are USB HID keycodes.

## Customization

### Changing Pin Assignments

To change the pin assignments, modify the pin numbers in the `main()` function in `src/main.rs`:

```rust
spawner.spawn(keyboard_task(
    p.PIN_9, p.PIN_8, p.PIN_7, p.PIN_6, p.PIN_5, p.PIN_4,  // Row pins
    p.PIN_26, p.PIN_27, p.PIN_28, p.PIN_29,                  // Column pins
)).unwrap();
```

### Adjusting Debounce Time

To change the debounce duration, modify the `DEBOUNCE_MS` constant:

```rust
const DEBOUNCE_MS: u64 = 10; // milliseconds
```

### Customizing the Display

The display initialization and rendering happens in the `display_task()` function. You can modify this to show different information like:
- Current layer
- Key press indicators
- Battery level
- Time/date

## Project Structure

```
.
├── .cargo/
│   └── config.toml          # Cargo build configuration
├── src/
│   └── main.rs              # Main firmware code
├── Cargo.toml               # Project dependencies
├── memory.x                 # Linker script for RP2040
└── README.md                # This file
```

## Development

### Viewing Logs

The firmware uses `defmt` for logging. To view logs during development with a debug probe:

```bash
cargo run --release
```

Logs will be displayed in the terminal via RTT (Real-Time Transfer).

### Common Issues

**Problem:** Compilation fails with linker errors
**Solution:** Make sure you have `flip-link` installed: `cargo install flip-link`

**Problem:** USB device not recognized
**Solution:** Check that your USB cable supports data transfer (not just charging)

**Problem:** Keys not registering
**Solution:** Verify your matrix wiring matches the pin configuration in the code

**Problem:** Display shows garbage
**Solution:** Check SPI connections and ensure the display is properly powered (3.3V)

## Learning Resources

- [Embassy Book](https://embassy.dev/book/) - Official Embassy framework documentation
- [RP2040 Datasheet](https://datasheets.raspberrypi.com/rp2040/rp2040-datasheet.pdf) - Hardware reference
- [USB HID Usage Tables](https://www.usb.org/sites/default/files/documents/hut1_12v2.pdf) - HID keycodes reference
- [Embedded Graphics](https://docs.rs/embedded-graphics/) - Display graphics library

## Understanding the Code

### Async Tasks

The firmware uses Embassy's async/await to run multiple tasks concurrently:

- **`keyboard_task()`**: Scans the keyboard matrix and detects key presses
- **`display_task()`**: Manages the OLED display
- **`usb_task()`**: Handles USB communication

These tasks run independently and efficiently share the CPU.

### Matrix Scanning

The keyboard matrix is scanned by:
1. Setting one row LOW (all others HIGH)
2. Reading all column pins (pulled HIGH by default)
3. If a column reads LOW, the key at that intersection is pressed
4. Repeat for all rows

This reduces the number of GPIO pins needed: 4 + 6 = 10 pins for 24 keys instead of 24 pins.

### Debouncing

Key switches produce electrical noise when pressed/released. The firmware implements software debouncing by requiring a key state to be stable for 10ms before registering a change.

## License

This project is dual-licensed under MIT OR Apache-2.0.

## Contributing

Contributions are welcome! Please feel free to submit issues or pull requests.
