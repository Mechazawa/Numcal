#![no_std]
#![no_main]

mod display;
mod keyboard;
mod modes;
mod usb;

use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{Input, Level, Output, Pull};
use embassy_rp::peripherals::USB;
use embassy_rp::spi::{Config as SpiConfig, Spi};
use embassy_rp::usb::{Driver, InterruptHandler};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::Timer;
use embassy_usb::class::cdc_acm::{CdcAcmClass, State as CdcState};
use embassy_usb::class::hid::{HidReaderWriter, State};
use embassy_usb::{Builder, Config as UsbConfig};
use embedded_hal_bus::spi::ExclusiveDevice;
use ssd1306::prelude::*;
use ssd1306::Ssd1306;
use static_cell::StaticCell;
use usbd_hid::descriptor::{KeyboardReport, SerializedDescriptor};
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => InterruptHandler<USB>;
});

// Keyboard matrix configuration
const ROWS: usize = 6;
const COLS: usize = 4;
const DEBOUNCE_MS: u8 = 10;

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

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    log::info!("NumCal starting...");

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
    static CDC_STATE: StaticCell<CdcState> = StaticCell::new();
    static REQUEST_HANDLER: StaticCell<usb::MyRequestHandler> = StaticCell::new();

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
        request_handler: Some(REQUEST_HANDLER.init(usb::MyRequestHandler {})),
        poll_ms: 60,
        max_packet_size: 8,
    };

    let hid = HidReaderWriter::<_, 1, 8>::new(&mut builder, HID_STATE.init(State::new()), hid_config);

    // Create CDC-ACM class for USB serial logging
    let cdc = CdcAcmClass::new(&mut builder, CDC_STATE.init(CdcState::new()), 64);

    // Build the USB device
    let usb = builder.build();

    log::info!("USB device configured as NumCal Keyboard");

    // Spawn USB tasks
    spawner.spawn(usb::usb_device_task(usb).unwrap());
    spawner.spawn(usb::usb_hid_task(hid).unwrap());
    spawner.spawn(logger_task(cdc).unwrap());

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
    static DISPLAY: StaticCell<display::DisplayType> = StaticCell::new();
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
    spawner.spawn(display::display_task(display).unwrap());
    spawner.spawn(keyboard::keyboard_task(rows, cols).unwrap());

    log::info!("All tasks spawned - NumCal ready!");

    // Main task just keeps the executor alive
    loop {
        Timer::after_secs(60).await;
    }
}

#[embassy_executor::task]
async fn logger_task(class: CdcAcmClass<'static, Driver<'static, USB>>) {
    embassy_usb_logger::with_class!(1024, log::LevelFilter::Info, class).await;
}
