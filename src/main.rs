#![no_std]
#![no_main]

// RP2040 boot2 bootloader - required for the chip to boot
// This uses the W25Q080 flash chip bootloader
#[link_section = ".boot2"]
#[used]
pub static BOOT2: [u8; 256] = rp2040_boot2::BOOT_LOADER_W25Q080;

use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{Input, Level, Output, Pull};
use embassy_rp::peripherals::{SPI1, USB};
use embassy_rp::spi::{Config as SpiConfig, Spi};
use embassy_rp::usb::{Driver, InterruptHandler as UsbInterruptHandler};
use embassy_time::{Duration, Instant, Timer};
use embassy_usb::class::hid::{HidReaderWriter, State as HidState};
use embassy_usb::{Builder, Config as UsbConfig};
use embedded_graphics::mono_font::ascii::FONT_6X10;
use embedded_graphics::mono_font::MonoTextStyleBuilder;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::text::{Baseline, Text};
use embedded_hal_bus::spi::ExclusiveDevice;
use heapless::Vec;
use ssd1306::prelude::*;
use ssd1306::Ssd1306;
use static_cell::StaticCell;
use usbd_hid::descriptor::{KeyboardReport, SerializedDescriptor};
use {defmt_rtt as _, panic_probe as _};

// Bind USB interrupt handler
bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => UsbInterruptHandler<USB>;
});

// Constants for the keyboard matrix
const ROWS: usize = 6;
const COLS: usize = 4;
const DEBOUNCE_MS: u64 = 10;

// Keymap: Maps each key in the 4x6 matrix to a USB HID keycode
// Layout is row-major: [row][col]
const KEYMAP: [[u8; COLS]; ROWS] = [
    [0x27, 0x25, 0x26, 0x24], // Row 0: 0, 8, 9, 7 (numpad keys)
    [0x1E, 0x22, 0x23, 0x21], // Row 1: 1, 5, 6, 4
    [0x1F, 0x1D, 0x20, 0x14], // Row 2: 2, *, 3, q
    [0x28, 0x2C, 0x51, 0x52], // Row 3: Enter, Space, Down, Up
    [0x4F, 0x50, 0x2A, 0x29], // Row 4: Right, Left, Backspace, Esc
    [0x2B, 0x39, 0x00, 0x00], // Row 5: Tab, CapsLock, unused, unused
];

// Key state structure with debouncing
#[derive(Clone, Copy)]
struct KeyState {
    pressed: bool,
    last_change: Instant,
}

impl KeyState {
    const fn new() -> Self {
        Self {
            pressed: false,
            last_change: Instant::from_ticks(0),
        }
    }
}

/// Main entry point - spawns all async tasks
#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Starting NumCal keyboard firmware");

    // Initialize the RP2040 peripherals with default configuration
    let p = embassy_rp::init(Default::default());

    // Initialize the OLED display task
    // Note: SPI1 pins are CLK=PIN_14, MOSI=PIN_15, CS=PIN_10, DC=PIN_13, RST=PIN_3
    spawner.spawn(display_task(p.SPI1, p.PIN_14, p.PIN_15, p.PIN_10, p.PIN_13, p.PIN_3)).unwrap();

    // Initialize the USB HID keyboard task
    // spawner.spawn(usb_task(p.USB)).unwrap();

    // Initialize the keyboard matrix scanner task
    // spawner.spawn(keyboard_task(
    //     p.PIN_9, p.PIN_8, p.PIN_7, p.PIN_6, p.PIN_5, p.PIN_4,  // Row pins
    //     p.PIN_26, p.PIN_27, p.PIN_28, p.PIN_29,                  // Column pins
    // )).unwrap();

    // Keep the main task alive
    loop {
        Timer::after(Duration::from_secs(60)).await;
    }
}

/// Keyboard matrix scanning task
///
/// This task continuously scans the 4x6 keyboard matrix, detects key presses,
/// and sends HID reports over USB. It implements debouncing to prevent false
/// key press detections.
///
/// Matrix scanning works by:
/// 1. Setting one row LOW at a time (all others HIGH)
/// 2. Reading all column pins (pulled HIGH by default)
/// 3. If a column reads LOW, the key at that row/col intersection is pressed
/// 4. Repeat for all rows
#[embassy_executor::task]
async fn keyboard_task(
    row0: embassy_rp::peripherals::PIN_9,
    row1: embassy_rp::peripherals::PIN_8,
    row2: embassy_rp::peripherals::PIN_7,
    row3: embassy_rp::peripherals::PIN_6,
    row4: embassy_rp::peripherals::PIN_5,
    row5: embassy_rp::peripherals::PIN_4,
    col0: embassy_rp::peripherals::PIN_26,
    col1: embassy_rp::peripherals::PIN_27,
    col2: embassy_rp::peripherals::PIN_28,
    col3: embassy_rp::peripherals::PIN_29,
) {
    info!("Initializing keyboard matrix scanner");

    // Configure row pins as outputs (drive LOW to scan, HIGH otherwise)
    let mut rows = [
        Output::new(row0, Level::High),
        Output::new(row1, Level::High),
        Output::new(row2, Level::High),
        Output::new(row3, Level::High),
        Output::new(row4, Level::High),
        Output::new(row5, Level::High),
    ];

    // Configure column pins as inputs with pull-up resistors
    // When a key is pressed, the column will be pulled LOW by the active row
    let cols = [
        Input::new(col0, Pull::Up),
        Input::new(col1, Pull::Up),
        Input::new(col2, Pull::Up),
        Input::new(col3, Pull::Up),
    ];

    // Track the state of each key for debouncing
    let mut key_states = [[KeyState::new(); COLS]; ROWS];

    info!("Keyboard matrix scanner initialized");

    loop {
        let now = Instant::now();
        let mut keys_pressed: Vec<u8, 6> = Vec::new(); // USB HID supports up to 6 simultaneous keys

        // Scan each row
        for (row_idx, row) in rows.iter_mut().enumerate() {
            // Set current row LOW to scan it
            row.set_low();

            // Small delay to allow the signal to stabilize
            Timer::after(Duration::from_micros(10)).await;

            // Read all columns
            for (col_idx, col) in cols.iter().enumerate() {
                // Column reads LOW when key is pressed
                let is_pressed = col.is_low();
                let key_state = &mut key_states[row_idx][col_idx];

                // Check if enough time has passed since last state change (debouncing)
                let debounce_elapsed = now.duration_since(key_state.last_change).as_millis() >= DEBOUNCE_MS;

                // Update key state if it changed and debounce time has passed
                if is_pressed != key_state.pressed && debounce_elapsed {
                    key_state.pressed = is_pressed;
                    key_state.last_change = now;

                    if is_pressed {
                        info!("Key pressed: row={}, col={}", row_idx, col_idx);
                    } else {
                        info!("Key released: row={}, col={}", row_idx, col_idx);
                    }
                }

                // If key is currently pressed and not 0 (unused), add to report
                if key_state.pressed {
                    let keycode = KEYMAP[row_idx][col_idx];
                    if keycode != 0 && keys_pressed.len() < 6 {
                        let _ = keys_pressed.push(keycode);
                    }
                }
            }

            // Set row back to HIGH
            row.set_high();
        }

        // Send HID report (this will be implemented with the USB task)
        // For now, we just log the pressed keys
        if !keys_pressed.is_empty() {
            info!("Keys pressed: {:?}", keys_pressed.as_slice());
        }

        // Small delay between scans to reduce CPU usage
        Timer::after(Duration::from_millis(1)).await;
    }
}

/// OLED display task
///
/// Initializes and manages the SSD1305 OLED display over SPI.
/// This task displays "Hello World" on the screen and could be extended
/// to show keyboard status, layer information, etc.
#[embassy_executor::task]
async fn display_task(
    spi_peripheral: SPI1,
    sck: embassy_rp::peripherals::PIN_14,
    mosi: embassy_rp::peripherals::PIN_15,
    cs: embassy_rp::peripherals::PIN_10,
    dc: embassy_rp::peripherals::PIN_13,
    rst: embassy_rp::peripherals::PIN_3,
) {
    info!("Initializing OLED display");

    // Configure SPI for the display
    // SSD1305 supports up to 10MHz SPI clock
    let mut spi_config = SpiConfig::default();
    spi_config.frequency = 8_000_000; // 8 MHz

    let spi = Spi::new_blocking_txonly(spi_peripheral, sck, mosi, spi_config);

    // Configure control pins
    let dc_pin = Output::new(dc, Level::Low);
    let mut rst_pin = Output::new(rst, Level::High);
    let cs_pin = Output::new(cs, Level::High); // Active low

    // Reset the display
    rst_pin.set_low();
    Timer::after(Duration::from_millis(10)).await;
    rst_pin.set_high();
    Timer::after(Duration::from_millis(10)).await;

    // Wrap SPI with ExclusiveDevice to provide SpiDevice trait
    let spi_device = ExclusiveDevice::new_no_delay(spi, cs_pin).unwrap();

    // Create the display interface
    let interface = ssd1306::prelude::SPIInterface::new(spi_device, dc_pin);

    // Initialize the SSD1305 driver (compatible with SSD1306 driver)
    let mut display = Ssd1306::new(interface, DisplaySize128x32, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();

    // Initialize the display
    display.init().unwrap();

    info!("OLED display initialized");

    // Create text style
    let text_style = MonoTextStyleBuilder::new()
        .font(&FONT_6X10)
        .text_color(BinaryColor::On)
        .build();

    // Draw "Hello World" text
    display.clear_buffer();
    Text::with_baseline("Hello World!", Point::new(20, 16), text_style, Baseline::Middle)
        .draw(&mut display)
        .unwrap();
    display.flush().unwrap();

    info!("Display updated with 'Hello World!'");

    // Keep the task alive
    loop {
        Timer::after(Duration::from_secs(1)).await;
    }
}

/// USB HID keyboard task
///
/// This task manages the USB connection and HID keyboard interface.
/// It creates a USB device that appears as a standard keyboard to the host.
///
/// In a complete implementation, this would receive key events from the
/// keyboard_task and send appropriate HID reports.
#[embassy_executor::task]
async fn usb_task(usb: USB) {
    info!("Initializing USB HID keyboard");

    // Create the USB driver
    let driver = Driver::new(usb, Irqs);

    // USB configuration
    let mut config = UsbConfig::new(0x16c0, 0x27dd); // Generic USB VID/PID
    config.manufacturer = Some("NumCal");
    config.product = Some("NumCal Keyboard");
    config.serial_number = Some("12345678");
    config.max_power = 100; // 100mA
    config.max_packet_size_0 = 64;

    // Required for Windows support
    config.device_class = 0x00;
    config.device_sub_class = 0x00;
    config.device_protocol = 0x00;
    config.composite_with_iads = false;

    // USB buffers (must be 'static)
    static CONFIG_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
    static BOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
    static MSOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
    static CONTROL_BUF: StaticCell<[u8; 128]> = StaticCell::new();
    static HID_STATE: StaticCell<HidState> = StaticCell::new();

    let config_descriptor = CONFIG_DESCRIPTOR.init([0; 256]);
    let bos_descriptor = BOS_DESCRIPTOR.init([0; 256]);
    let msos_descriptor = MSOS_DESCRIPTOR.init([0; 256]);
    let control_buf = CONTROL_BUF.init([0; 128]);
    let hid_state = HID_STATE.init(HidState::new());

    let mut builder = Builder::new(
        driver,
        config,
        config_descriptor,
        bos_descriptor,
        msos_descriptor,
        control_buf,
    );

    // Create HID keyboard class
    let hid_config = embassy_usb::class::hid::Config {
        report_descriptor: KeyboardReport::desc(),
        request_handler: None,
        poll_ms: 10,
        max_packet_size: 64,
    };

    let hid = HidReaderWriter::<_, 1, 8>::new(&mut builder, hid_state, hid_config);

    // Build the USB device
    let mut usb = builder.build();

    info!("USB device configured, starting...");

    // Run the USB device
    let usb_fut = usb.run();

    // HID report sending loop
    let (reader, mut writer) = hid.split();

    // Spawn a task to handle USB events
    let hid_fut = async {
        // Drop reader as we don't need to receive reports
        drop(reader);

        loop {
            Timer::after(Duration::from_millis(10)).await;

            // In a complete implementation, this would receive key events
            // from a channel shared with keyboard_task and send appropriate reports

            // Example: Send empty report (no keys pressed)
            let report = KeyboardReport {
                modifier: 0,
                reserved: 0,
                leds: 0,
                keycodes: [0; 6],
            };

            // Send the report
            match writer.write_serialize(&report).await {
                Ok(()) => {}
                Err(e) => {
                    warn!("Failed to send HID report: {:?}", e);
                }
            }
        }
    };

    info!("USB HID keyboard ready");

    // Run both futures concurrently
    embassy_futures::join::join(usb_fut, hid_fut).await;
}
