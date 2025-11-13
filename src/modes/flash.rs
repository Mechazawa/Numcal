use embassy_time::Timer;
use embedded_graphics::{Drawable, Pixel};
use embedded_graphics::geometry::{Dimensions, Point};
use embedded_graphics::mono_font::ascii::FONT_6X10;
use embedded_graphics::mono_font::MonoTextStyleBuilder;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::{DrawTarget, PointsIter};
use embedded_graphics::text::{Baseline, Text};
use crate::modes::{Mode, MODE_RUNNING};
use crate::tasks::DisplayProxy;
use portable_atomic::{Ordering};
use crate::show_text;

pub struct FlashMode{}

const SQUARE_SIZE: i32 = 8;

impl FlashMode {
    pub fn new() -> Self {
        Self {}
    }
}

impl Mode for FlashMode {
    async fn task(&mut self) {
        let mut display = DisplayProxy::new();

        show_text("Flash test");

        Timer::after_secs(1).await;

        let mut inverted = false;

        while MODE_RUNNING.load(Ordering::Relaxed) {
            // display.draw_iter(
            //     display
            //         .bounding_box()
            //         .points()
            //         .map(|point| Pixel(point, {
            //             let mut color = inverted;
            //
            //             if (point.x / SQUARE_SIZE) % 2 == 1 {
            //                 color = !color;
            //             }
            //
            //             if (point.y / SQUARE_SIZE) % 2 == 1 {
            //                 color = !color;
            //             }
            //
            //             if color {
            //                 BinaryColor::Off
            //             } else {
            //                 BinaryColor::On
            //             }
            //         }))
            // ).unwrap();

            display.clear(if inverted {BinaryColor::On} else {BinaryColor::Off}).unwrap();
            display.flush().unwrap();

            Timer::after_secs(1).await;
            inverted = !inverted;
        }
    }
}