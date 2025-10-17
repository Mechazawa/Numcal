#![no_std]
#![no_main]

use core::fmt::Write;
use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{Level, Output};
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

// Channel for sending display updates from counter task to display task
static DISPLAY_CHANNEL: Channel<ThreadModeRawMutex, heapless::String<32>, 2> = Channel::new();

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
async fn counter_task() {
    log::info!("Counter task started");

    let sender = DISPLAY_CHANNEL.sender();

    // Send initial message
    let mut text = heapless::String::<32>::new();
    write!(&mut text, "Hello OLED!").unwrap();
    sender.send(text).await;

    // Increment counter every 5 seconds
    let mut counter = 0;
    loop {
        Timer::after_secs(5).await;
        counter += 1;

        let mut text = heapless::String::<32>::new();
        write!(&mut text, "Count: {counter}").unwrap();
        sender.send(text).await;

        log::info!("Counter: {counter}");
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

    // Spawn tasks
    spawner.spawn(display_task(display).unwrap());
    spawner.spawn(counter_task().unwrap());

    // Main task just keeps the executor alive
    loop {
        Timer::after_secs(60).await;
    }
}
