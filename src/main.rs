#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{SPI1, USB};
use embassy_rp::spi::{Config as SpiConfig, Spi};
use embassy_rp::usb::{Driver, InterruptHandler};
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
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => InterruptHandler<USB>;
});

#[embassy_executor::task]
async fn logger_task(driver: Driver<'static, USB>) {
    embassy_usb_logger::run!(1024, log::LevelFilter::Info, driver);
}


#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    let driver = Driver::new(p.USB, Irqs);
    spawner.spawn(logger_task(driver).expect("Failed to spawn logger task"));

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

    // Initialize the SSD1306 driver (128x64) - trying 64 height to fix interlacing
    log::info!("Creating display driver (128x64)...");
    let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();

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

    // Create text style
    let text_style = MonoTextStyleBuilder::new()
        .font(&FONT_6X10)
        .text_color(BinaryColor::On)
        .build();

    // Draw "Hello OLED!" text - moved down to avoid top cutoff
    log::info!("Drawing text to display...");
    display.clear(BinaryColor::Off).unwrap();

    // Clear the rightmost columns explicitly to remove gibberish
    Rectangle::new(Point::new(124, 0), Size::new(4, 64))
        .into_styled(PrimitiveStyle::with_fill(BinaryColor::Off))
        .draw(&mut display)
        .unwrap();

    Text::with_baseline("Hello OLED!", Point::new(5, 38), text_style, Baseline::Middle)
        .draw(&mut display)
        .unwrap();

    match display.flush() {
        Ok(_) => log::info!("Display updated successfully!"),
        Err(_) => log::error!("Display flush failed!"),
    }

    log::info!("Display initialization complete, entering update loop");

    // Keep the task alive and update counter every 5 seconds
    let mut counter = 0;
    loop {
        Timer::after_secs(5).await;
        counter += 1;

        display.clear(BinaryColor::Off).unwrap();

        // Clear the rightmost columns explicitly to remove gibberish
        Rectangle::new(Point::new(124, 0), Size::new(4, 64))
            .into_styled(PrimitiveStyle::with_fill(BinaryColor::Off))
            .draw(&mut display)
            .unwrap();

        let mut text = heapless::String::<32>::new();
        use core::fmt::Write;
        write!(&mut text, "Count: {}", counter).unwrap();

        Text::with_baseline(&text, Point::new(5, 38), text_style, Baseline::Middle)
            .draw(&mut display)
            .unwrap();
        display.flush().unwrap();

        log::info!("Display updated: {}", counter);
    }
}
