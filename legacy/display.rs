use embassy_rp::gpio::Output;
use embassy_rp::spi::Spi;
use embedded_graphics::mono_font::ascii::FONT_6X10;
use embedded_graphics::mono_font::MonoTextStyleBuilder;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyle, Rectangle};
use embedded_graphics::text::{Baseline, Text};
use embedded_hal_bus::spi::ExclusiveDevice;
use log::trace;
use ssd1306::prelude::*;
use ssd1306::Ssd1306;

use crate::DISPLAY_CHANNEL;

pub type DisplayType = Ssd1306<
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
pub async fn display_task(display: &'static mut DisplayType) {
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
        trace!("Rendering: {}", text.as_str());

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
            Ok(_) => trace!("Display updated successfully"),
            Err(_) => log::error!("Display flush failed!"),
        }
    }
}
