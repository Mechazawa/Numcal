use embassy_time::Timer;
use embedded_graphics::Drawable;
use embedded_graphics::geometry::Point;
use embedded_graphics::mono_font::ascii::FONT_6X10;
use embedded_graphics::mono_font::MonoTextStyleBuilder;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::DrawTarget;
use embedded_graphics::text::{Baseline, Text};
use crate::modes::{Mode};
use crate::tasks::DisplayProxy;

pub struct BootMode{}

impl BootMode {
    pub fn new() -> Self {
        Self {}
    }
}

impl Mode for BootMode {
    async fn task(&mut self) {
        let mut display = DisplayProxy::new();

        let text_style = MonoTextStyleBuilder::new()
            .font(&FONT_6X10)
            .text_color(BinaryColor::On)
            .build();

        display.clear(BinaryColor::Off).unwrap();

        Text::with_baseline("NumCal", Point::new(5, 38), text_style, Baseline::Middle)
            .draw(&mut display)
            .unwrap();

        display.flush().unwrap();

        Timer::after_secs(2).await;
    }
}