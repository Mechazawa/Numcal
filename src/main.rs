#![no_std]
#![no_main]

use core::fmt::Write;
use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{Input, Level, Output, Pull};
use embassy_rp::peripherals::USB;
use embassy_rp::spi::{Config as SpiConfig, Spi};
use embassy_rp::usb::{Driver, InterruptHandler};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::Timer;
use embassy_usb::class::hid::{HidReaderWriter, ReportId, RequestHandler, State};
use embassy_usb::{Builder, Config as UsbConfig};
use usbd_hid::descriptor::{KeyboardReport, SerializedDescriptor};
use embedded_graphics::mono_font::ascii::FONT_6X10;
use embedded_graphics::mono_font::MonoTextStyleBuilder;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyle, Rectangle};
use embedded_graphics::text::{Baseline, Text};
use embedded_hal_bus::spi::ExclusiveDevice;
use ssd1306::prelude::*;
use ssd1306::Ssd1306;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => InterruptHandler<USB>;
});

// Keyboard matrix configuration
const ROWS: usize = 6;
const COLS: usize = 4;
const DEBOUNCE_MS: u64 = 10;

// Keymap: USB HID keycodes for each matrix position
// Based on TODO.md numpad layout
// 0x00 = no key/unused position
const KEYMAP: [[u8; COLS]; ROWS] = [
    [0x00, 0x00, 0x00, 0x00],       // Row 0: Reserved for mode switching
    [0x53, 0x54, 0x55, 0x56],       // Row 1: Numlock, /, *, -
    [0x5F, 0x60, 0x61, 0x00],       // Row 2: 7, 8, 9, unused
    [0x5C, 0x5D, 0x5E, 0x57],       // Row 3: 4, 5, 6, +
    [0x59, 0x5A, 0x5B, 0x00],       // Row 4: 1, 2, 3, unused
    [0x00, 0x62, 0x63, 0x58],       // Row 5: unused, 0, ., Enter
];

// Key event for USB HID communication
#[derive(Clone, Copy, Debug)]
struct KeyEvent {
    row: usize,
    col: usize,
    pressed: bool,
}

// Channel for sending display updates from keyboard task to display task
static DISPLAY_CHANNEL: Channel<ThreadModeRawMutex, heapless::String<64>, 2> = Channel::new();

// Channel for sending key events from keyboard task to USB HID task
static USB_CHANNEL: Channel<ThreadModeRawMutex, KeyEvent, 8> = Channel::new();

type DisplayType = Ssd1306<
    SPIInterface<
        ExclusiveDevice<
            Spi<'static, embassy_rp::peripherals::SPI1, embassy_rp::spi::Blocking>,
            Output<'static>,
            embedded_hal_bus::spi::NoDelay,
        >,
        Output<'static>,
    >,
    DisplaySize128x64,
    ssd1306::mode::BufferedGraphicsMode<DisplaySize128x64>,
>;

// Empty request handler for HID
struct MyRequestHandler {}

impl RequestHandler for MyRequestHandler {
    fn get_report(&mut self, id: ReportId, _buf: &mut [u8]) -> Option<usize> {
        defmt::info!("Get report for {:?}", id);
        None
    }

    fn set_report(&mut self, id: ReportId, data: &[u8]) -> embassy_usb::control::OutResponse {
        defmt::info!("Set report for {:?}: {:?}", id, data);
        embassy_usb::control::OutResponse::Accepted
    }

    fn set_idle_ms(&mut self, id: Option<ReportId>, dur: u32) {
        defmt::info!("Set idle rate for {:?} to {:?}", id, dur);
    }

    fn get_idle_ms(&mut self, id: Option<ReportId>) -> Option<u32> {
        defmt::info!("Get idle rate for {:?}", id);
        None
    }
}

#[embassy_executor::task]
async fn usb_device_task(mut usb: embassy_usb::UsbDevice<'static, Driver<'static, USB>>) {
    usb.run().await;
}

#[embassy_executor::task]
async fn usb_hid_task(
    mut writer: HidReaderWriter<'static, Driver<'static, USB>, 1, 8>,
) {
    let usb_sender = USB_CHANNEL.receiver();

    // Track currently pressed keys (max 6 keys for NKRO)
    let mut pressed_keys: heapless::Vec<u8, 6> = heapless::Vec::new();

    defmt::info!("USB HID task started");

    loop {
        // Wait for key event
        let event = usb_sender.receive().await;
        defmt::info!("USB HID: Key event R{}C{} pressed={}", event.row, event.col, event.pressed);

        let keycode = KEYMAP[event.row][event.col];

        // Skip if keycode is 0x00 (unused key)
        if keycode == 0x00 {
            continue;
        }

        // Update pressed keys list
        if event.pressed {
            // Add key if not already in list and there's space
            if !pressed_keys.contains(&keycode) && pressed_keys.len() < 6 {
                let _ = pressed_keys.push(keycode);
                defmt::info!("USB HID: Added keycode 0x{:02x}, now {} keys pressed", keycode, pressed_keys.len());
            }
        } else {
            // Remove key from list
            if let Some(pos) = pressed_keys.iter().position(|&k| k == keycode) {
                pressed_keys.swap_remove(pos);
                defmt::info!("USB HID: Removed keycode 0x{:02x}, now {} keys pressed", keycode, pressed_keys.len());
            }
        }

        // Build HID report
        let mut report = KeyboardReport {
            modifier: 0,
            reserved: 0,
            leds: 0,
            keycodes: [0u8; 6],
        };

        // Copy pressed keys into report
        for (i, &keycode) in pressed_keys.iter().enumerate() {
            report.keycodes[i] = keycode;
        }

        defmt::info!("USB HID: Sending report with {} keys", pressed_keys.len());

        // Send report
        match writer.write_serialize(&report).await {
            Ok(()) => {
                defmt::info!("USB HID: Report sent successfully");
            }
            Err(e) => {
                defmt::error!("USB HID: Failed to send report: {:?}", e);
            }
        }
    }
}

#[embassy_executor::task]
async fn keyboard_task(
    rows: &'static mut [Output<'static>; ROWS],
    cols: &'static [Input<'static>; COLS],
) {
    defmt::info!("Keyboard task started");

    // Track key states for debouncing: [row][col] -> (is_pressed, debounce_timer)
    let mut key_states = [[false; COLS]; ROWS];
    let mut debounce_timers = [[0u64; COLS]; ROWS];

    let display_sender = DISPLAY_CHANNEL.sender();
    let usb_sender = USB_CHANNEL.sender();

    defmt::info!("Keyboard matrix scanner initialized");

    // Debug: Check initial column states
    Timer::after_millis(100).await;
    for (idx, col) in cols.iter().enumerate() {
        defmt::info!("Initial col {} state: high={}", idx, col.is_high());
    }

    loop {
        let mut pressed_keys = heapless::Vec::<(usize, usize), 24>::new(); // Max 24 keys (6x4)

        // Scan the matrix
        for (row_idx, row_pin) in rows.iter_mut().enumerate() {
            // Drive this row LOW
            row_pin.set_low();

            // Small delay to let the signal settle
            Timer::after_micros(10).await;

            // Read all columns
            for (col_idx, col_pin) in cols.iter().enumerate() {
                let is_low = col_pin.is_low();

                // Debug logging when we see a LOW column (only on first detection)
                if is_low && debounce_timers[row_idx][col_idx] == 0 && !key_states[row_idx][col_idx] {
                    defmt::trace!("Detected LOW at R{}C{}", row_idx, col_idx);
                }

                // Update debounce logic
                if is_low != key_states[row_idx][col_idx] {
                    // State differs from stable state
                    if debounce_timers[row_idx][col_idx] == 0 {
                        // Start debounce timer
                        debounce_timers[row_idx][col_idx] = DEBOUNCE_MS;
                    } else {
                        // Decrement timer
                        debounce_timers[row_idx][col_idx] -= 1;

                        // Timer expired, update stable state
                        if debounce_timers[row_idx][col_idx] == 0 {
                            let was_pressed = key_states[row_idx][col_idx];
                            key_states[row_idx][col_idx] = is_low;

                            if is_low && !was_pressed {
                                defmt::info!("Key pressed: R{}C{} (keycode=0x{:02x})", row_idx, col_idx, KEYMAP[row_idx][col_idx]);
                                // Send key press event to USB
                                usb_sender.send(KeyEvent {
                                    row: row_idx,
                                    col: col_idx,
                                    pressed: true,
                                }).await;
                            } else if !is_low && was_pressed {
                                defmt::info!("Key released: R{}C{} (keycode=0x{:02x})", row_idx, col_idx, KEYMAP[row_idx][col_idx]);
                                // Send key release event to USB
                                usb_sender.send(KeyEvent {
                                    row: row_idx,
                                    col: col_idx,
                                    pressed: false,
                                }).await;
                            }
                        }
                    }
                } else {
                    // State matches stable state, reset timer
                    debounce_timers[row_idx][col_idx] = 0;
                }

                // Collect currently pressed keys
                if key_states[row_idx][col_idx] {
                    let _ = pressed_keys.push((row_idx, col_idx));
                }
            }

            // Set row back to HIGH
            row_pin.set_high();
        }

        // Send display update
        let mut text = heapless::String::<64>::new();
        if pressed_keys.is_empty() {
            write!(&mut text, "No keys").unwrap();
        } else {
            for (i, (row, col)) in pressed_keys.iter().enumerate() {
                if i > 0 {
                    write!(&mut text, " ").unwrap();
                }
                write!(&mut text, "R{row}C{col}").unwrap();
            }
        }
        display_sender.send(text).await;

        // Scan rate: 1ms between scans
        Timer::after_millis(1).await;
    }
}

#[embassy_executor::task]
async fn display_task(display: &'static mut DisplayType) {
    defmt::info!("Display rendering task started");

    let receiver = DISPLAY_CHANNEL.receiver();

    // Create text style
    let text_style = MonoTextStyleBuilder::new()
        .font(&FONT_6X10)
        .text_color(BinaryColor::On)
        .build();

    // Wait for messages and render them
    loop {
        let text = receiver.receive().await;
        defmt::trace!("Rendering: {}", text.as_str());

        // Clear display
        display.clear(BinaryColor::Off).unwrap();

        // Clear the rightmost columns explicitly to remove gibberish
        Rectangle::new(Point::new(124, 0), Size::new(4, 64))
            .into_styled(PrimitiveStyle::with_fill(BinaryColor::Off))
            .draw(display)
            .unwrap();

        // Draw text
        Text::with_baseline(text.as_str(), Point::new(5, 38), text_style, Baseline::Middle)
            .draw(display)
            .unwrap();

        // Flush to display
        match display.flush() {
            Ok(_) => defmt::trace!("Display updated successfully"),
            Err(_) => defmt::error!("Display flush failed!"),
        }
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    defmt::info!("NumCal starting...");

    // Set up USB driver
    let driver = Driver::new(p.USB, Irqs);

    // Create the USB device configuration
    let mut config = UsbConfig::new(0x16c0, 0x27dd);
    config.manufacturer = Some("NumCal");
    config.product = Some("NumCal Keyboard");
    config.serial_number = Some("12345678");
    config.max_power = 100;
    config.max_packet_size_0 = 64;

    // USB device and builder buffers
    static CONFIG_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
    static BOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
    static CONTROL_BUF: StaticCell<[u8; 64]> = StaticCell::new();
    static HID_STATE: StaticCell<State> = StaticCell::new();
    static REQUEST_HANDLER: StaticCell<MyRequestHandler> = StaticCell::new();

    let mut builder = Builder::new(
        driver,
        config,
        CONFIG_DESCRIPTOR.init([0; 256]),
        BOS_DESCRIPTOR.init([0; 256]),
        &mut [], // no msos descriptors
        CONTROL_BUF.init([0; 64]),
    );

    // Create HID class
    let hid_config = embassy_usb::class::hid::Config {
        report_descriptor: KeyboardReport::desc(),
        request_handler: Some(REQUEST_HANDLER.init(MyRequestHandler {})),
        poll_ms: 60,
        max_packet_size: 8,
    };

    let hid = HidReaderWriter::<_, 1, 8>::new(&mut builder, HID_STATE.init(State::new()), hid_config);

    // Build the USB device
    let usb = builder.build();

    defmt::info!("USB device configured as NumCal Keyboard");

    // Spawn USB tasks
    spawner.spawn(usb_device_task(usb).unwrap());
    spawner.spawn(usb_hid_task(hid).unwrap());

    defmt::info!("Initializing OLED display on SPI1");
    defmt::info!("Pins: CLK=14, MOSI=15, CS=10, DC=13, RST=3");

    // Configure SPI for the display
    let mut spi_config = SpiConfig::default();
    spi_config.frequency = 8_000_000; // 8 MHz
    defmt::info!("SPI frequency: 8 MHz");

    let spi = Spi::new_blocking_txonly(p.SPI1, p.PIN_14, p.PIN_15, spi_config);
    defmt::info!("SPI initialized");

    // Configure control pins
    let dc_pin = Output::new(p.PIN_13, Level::Low);
    let mut rst_pin = Output::new(p.PIN_3, Level::High);
    let cs_pin = Output::new(p.PIN_10, Level::High);
    defmt::info!("Control pins configured");

    // Reset the display
    defmt::info!("Resetting display...");
    rst_pin.set_low();
    Timer::after_millis(10).await;
    rst_pin.set_high();
    Timer::after_millis(10).await;
    defmt::info!("Reset complete");

    // Wrap SPI with ExclusiveDevice
    let spi_device = ExclusiveDevice::new_no_delay(spi, cs_pin).unwrap();
    defmt::info!("SPI device wrapped");

    // Create the display interface
    let interface = ssd1306::prelude::SPIInterface::new(spi_device, dc_pin);
    defmt::info!("Display interface created");

    // Initialize the SSD1306 driver (128x64)
    defmt::info!("Creating display driver (128x64)...");
    static DISPLAY: StaticCell<DisplayType> = StaticCell::new();
    let display = DISPLAY.init(
        Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
            .into_buffered_graphics_mode(),
    );

    // Initialize the display
    defmt::info!("Initializing display hardware...");
    match display.init() {
        Ok(_) => defmt::info!("Display initialized successfully!"),
        Err(_) => {
            defmt::error!("Display initialization failed!");
            loop {
                Timer::after_secs(1).await;
            }
        }
    }

    defmt::info!("Display initialization complete");

    // Initialize keyboard matrix GPIO
    defmt::info!("Initializing keyboard matrix");

    static ROWS_CELL: StaticCell<[Output<'static>; ROWS]> = StaticCell::new();
    let rows = ROWS_CELL.init([
        Output::new(p.PIN_9, Level::High),
        Output::new(p.PIN_8, Level::High),
        Output::new(p.PIN_7, Level::High),
        Output::new(p.PIN_6, Level::High),
        Output::new(p.PIN_5, Level::High),
        Output::new(p.PIN_4, Level::High),
    ]);

    static COLS_CELL: StaticCell<[Input<'static>; COLS]> = StaticCell::new();
    let cols = COLS_CELL.init([
        Input::new(p.PIN_26, Pull::Up),
        Input::new(p.PIN_27, Pull::Up),
        Input::new(p.PIN_28, Pull::Up),
        Input::new(p.PIN_29, Pull::Up),
    ]);

    defmt::info!("Keyboard matrix initialized");

    // Spawn tasks
    spawner.spawn(display_task(display).unwrap());
    spawner.spawn(keyboard_task(rows, cols).unwrap());

    defmt::info!("All tasks spawned - NumCal ready!");

    // Main task just keeps the executor alive
    loop {
        Timer::after_secs(60).await;
    }
}
