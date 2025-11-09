use embassy_executor::Spawner;
use embassy_rp::gpio::{Level, Output, Pin};
use embassy_rp::peripherals::SPI1;
use embassy_rp::spi::{ClkPin, Config as SpiConfig, MosiPin, Spi};
use embassy_rp::Peri;
use embassy_time::Timer;
use embedded_graphics::mono_font::ascii::FONT_6X10;
use embedded_graphics::mono_font::MonoTextStyleBuilder;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyle, Rectangle};
use embedded_graphics::text::{Baseline, Text};
use embedded_hal_bus::spi::ExclusiveDevice;
use log::{error, info};
use ssd1306::Ssd1306;
use ssd1306::prelude::*;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};
use crate::tasks::DISPLAY_CHANNEL;

// Display
type PinSpi = SPI1;
type DisplayType = Ssd1306<
    SPIInterface<
        ExclusiveDevice<
            Spi<'static, PinSpi, embassy_rp::spi::Blocking>,
            Output<'static>,
            embedded_hal_bus::spi::NoDelay,
        >,
        Output<'static>,
    >,
    DisplaySize128x64,
    ssd1306::mode::BufferedGraphicsMode<DisplaySize128x64>,
>;

static DISPLAY: StaticCell<DisplayType> = StaticCell::new();

pub async fn init(
    spawner: &Spawner,
    pin_spi: Peri<'static, PinSpi>,
    pin_clk: Peri<'static, impl ClkPin<PinSpi>>,
    pin_mosi: Peri<'static, impl MosiPin<PinSpi>>,
    pin_dc: Peri<'static, impl Pin>,
    pin_rst: Peri<'static, impl Pin>,
    pin_cs: Peri<'static, impl Pin>,
) {
    // Init SPI
    let mut spi_config = SpiConfig::default();
    spi_config.frequency = 8_000_000; // 8 MHz

    let spi = Spi::new_blocking_txonly(pin_spi, pin_clk, pin_mosi, spi_config);

    // Configure control pins
    let dc_pin = Output::new(pin_dc, Level::Low);
    let mut rst_pin = Output::new(pin_rst, Level::High);
    let cs_pin = Output::new(pin_cs, Level::High);

    // Reset the display
    rst_pin.set_low();
    Timer::after_millis(10).await;
    rst_pin.set_high();
    Timer::after_millis(10).await;

    // Create the display interface
    let spi_device = ExclusiveDevice::new_no_delay(spi, cs_pin).unwrap();
    let interface = SPIInterface::new(spi_device, dc_pin);

    // Initialize the SSD1306 driver (128x64)
    info!("Creating display driver (128x64)...");
    let display = DISPLAY.init(
        Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
            .into_buffered_graphics_mode(),
    );

    match display.init() {
        Ok(()) => {
            spawner.spawn(display_task(display).unwrap());

            info!("Display initialized successfully!");
        },
        Err(_) => {
            error!("Display initialization failed!");
        }
    }
}

#[embassy_executor::task]
async fn display_task(display: &'static mut DisplayType) {
    info!("Display rendering task started");

    let receiver = DISPLAY_CHANNEL.receiver();

    loop {
        let text = receiver.receive().await;

        // Create text style
        let text_style = MonoTextStyleBuilder::new()
            .font(&FONT_6X10)
            .text_color(BinaryColor::On)
            .build();

        display.clear(BinaryColor::Off).unwrap();

        // Draw text
        Text::with_baseline(text.as_str(), Point::new(5, 38), text_style, Baseline::Middle)
            .draw(display)
            .unwrap();

        info!("Text should be drawn");
    }
}
