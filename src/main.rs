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

// Keymap: maps matrix positions to key names (for reference)
// Based on README.md layout
const KEYMAP: [[&str; COLS]; ROWS] = [
    ["0", "8", "9", "7"],           // Row 0
    ["1", "5", "6", "4"],           // Row 1
    ["2", "*", "3", "Q"],           // Row 2
    ["Enter", "Space", "Down", "Up"],  // Row 3
    ["Right", "Left", "Bkspc", "Esc"], // Row 4
    ["Tab", "Caps", "-", "-"],      // Row 5
];

// Channel for sending display updates from keyboard task to display task
static DISPLAY_CHANNEL: Channel<ThreadModeRawMutex, heapless::String<64>, 2> = Channel::new();

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

#[embassy_executor::task]
async fn logger_task(driver: Driver<'static, USB>) {
    embassy_usb_logger::run!(1024, log::LevelFilter::Info, driver);
}

#[embassy_executor::task]
async fn keyboard_task(
    rows: &'static mut [Output<'static>; ROWS],
    cols: &'static [Input<'static>; COLS],
) {
    log::info!("Keyboard task started");

    // Track key states for debouncing: [row][col] -> (is_pressed, debounce_timer)
    let mut key_states = [[false; COLS]; ROWS];
    let mut debounce_timers = [[0u64; COLS]; ROWS];

    let sender = DISPLAY_CHANNEL.sender();

    log::info!("Keyboard matrix scanner initialized");

    // Debug: Check initial column states
    Timer::after_millis(100).await;
    for (idx, col) in cols.iter().enumerate() {
        log::info!("Initial col {} state: high={}", idx, col.is_high());
    }

    loop {
        let mut pressed_keys = heapless::Vec::<(usize, usize), 24>::new(); // Max 24 keys (6x4)
        let mut any_low = false;

        // Scan the matrix
        for (row_idx, row_pin) in rows.iter_mut().enumerate() {
            // Drive this row LOW
            row_pin.set_low();

            // Small delay to let the signal settle
            Timer::after_micros(10).await;

            // Read all columns
            for (col_idx, col_pin) in cols.iter().enumerate() {
                let is_low = col_pin.is_low();

                // Debug logging when we see a LOW column
                if is_low {
                    any_low = true;
                    if debounce_timers[row_idx][col_idx] == 0 {
                        log::info!("Detected LOW at R{}C{}", row_idx, col_idx);
                    }
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
                                log::info!("Key pressed: R{}C{} ({})", row_idx, col_idx, KEYMAP[row_idx][col_idx]);
                            } else if !is_low && was_pressed {
                                log::info!("Key released: R{}C{} ({})", row_idx, col_idx, KEYMAP[row_idx][col_idx]);
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
                write!(&mut text, "R{}C{}", row, col).unwrap();
            }
        }
        sender.send(text).await;

        // Debug: Log if we saw any LOW signals
        if any_low {
            log::info!("Scan detected {} LOW signal(s)", if pressed_keys.is_empty() { "debouncing" } else { "pressed" });
        }

        // Scan rate: 1ms between scans
        Timer::after_millis(1).await;
    }
}

#[embassy_executor::task]
async fn display_task(display: &'static mut DisplayType) {
    log::info!("Display rendering task started");

    let receiver = DISPLAY_CHANNEL.receiver();

    // Create text style
    let text_style = MonoTextStyleBuilder::new()
        .font(&FONT_6X10)
        .text_color(BinaryColor::On)
        .build();

    // Wait for messages and render them
    loop {
        let text = receiver.receive().await;
        log::info!("Rendering: {}", text.as_str());

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
            Ok(_) => log::info!("Display updated successfully"),
            Err(_) => log::error!("Display flush failed!"),
        }
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    let driver = Driver::new(p.USB, Irqs);
    spawner.spawn(logger_task(driver).unwrap());

    log::info!("NumCal starting...");

    // Wait a bit for USB logger to be ready
    Timer::after_millis(500).await;

    log::info!("Initializing OLED display on SPI1");
    log::info!("Pins: CLK=14, MOSI=15, CS=10, DC=13, RST=3");

    // Configure SPI for the display
    let mut spi_config = SpiConfig::default();
    spi_config.frequency = 8_000_000; // 8 MHz
    log::info!("SPI frequency: 8 MHz");

    let spi = Spi::new_blocking_txonly(p.SPI1, p.PIN_14, p.PIN_15, spi_config);
    log::info!("SPI initialized");

    // Configure control pins
    let dc_pin = Output::new(p.PIN_13, Level::Low);
    let mut rst_pin = Output::new(p.PIN_3, Level::High);
    let cs_pin = Output::new(p.PIN_10, Level::High);
    log::info!("Control pins configured");

    // Reset the display
    log::info!("Resetting display...");
    rst_pin.set_low();
    Timer::after_millis(10).await;
    rst_pin.set_high();
    Timer::after_millis(10).await;
    log::info!("Reset complete");

    // Wrap SPI with ExclusiveDevice
    let spi_device = ExclusiveDevice::new_no_delay(spi, cs_pin).unwrap();
    log::info!("SPI device wrapped");

    // Create the display interface
    let interface = ssd1306::prelude::SPIInterface::new(spi_device, dc_pin);
    log::info!("Display interface created");

    // Initialize the SSD1306 driver (128x64)
    log::info!("Creating display driver (128x64)...");
    static DISPLAY: StaticCell<DisplayType> = StaticCell::new();
    let display = DISPLAY.init(
        Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
            .into_buffered_graphics_mode(),
    );

    // Initialize the display
    log::info!("Initializing display hardware...");
    match display.init() {
        Ok(_) => log::info!("Display initialized successfully!"),
        Err(_) => {
            log::error!("Display initialization failed!");
            loop {
                Timer::after_secs(1).await;
            }
        }
    }

    log::info!("Display initialization complete");

    // Initialize keyboard matrix GPIO
    log::info!("Initializing keyboard matrix");

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

    log::info!("Keyboard matrix initialized");

    // Spawn tasks
    spawner.spawn(display_task(display).unwrap());
    spawner.spawn(keyboard_task(rows, cols).unwrap());

    log::info!("All tasks spawned");

    // Main task just keeps the executor alive
    loop {
        Timer::after_secs(60).await;
    }
}
