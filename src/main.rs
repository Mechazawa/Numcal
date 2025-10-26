#![no_std]
#![no_main]

mod tasks;

use embassy_executor::Spawner;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::spi::{Config as SpiConfig, Spi};
use embassy_time::Timer;
use embedded_hal_bus::spi::ExclusiveDevice;
use ssd1306::prelude::*;
use ssd1306::Ssd1306;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    defmt::info!("NumCal starting...");

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
    static DISPLAY: StaticCell<tasks::display::DisplayType> = StaticCell::new();
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

    // Spawn display task
    spawner.spawn(tasks::display::display_task(display).unwrap());

    defmt::info!("Display task spawned - NumCal ready!");

    // Main task just keeps the executor alive
    loop {
        Timer::after_secs(60).await;
    }
}
