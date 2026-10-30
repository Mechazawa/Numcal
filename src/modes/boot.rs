use embassy_time::Timer;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::mono_font::ascii::{FONT_6X10, FONT_10X20};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyle, Rectangle};
use embedded_graphics::text::{Baseline, Text};

use crate::modes::Mode;
use crate::tasks::DisplayProxy;

const FRAMES: i32 = 20;
const TITLE_LEFT: i32 = 26;

pub struct Boot {}

impl Boot {
    pub fn new() -> Self {
        Self {}
    }

    fn draw_icon(display: &mut DisplayProxy) {
        let outline = PrimitiveStyle::with_stroke(BinaryColor::On, 1);

        Rectangle::new(Point::new(2, 2), Size::new(18, 28))
            .into_styled(outline)
            .draw(display)
            .unwrap();
        display
            .fill_solid(&Rectangle::new(Point::new(5, 5), Size::new(12, 6)), BinaryColor::On)
            .unwrap();

        let keys = (0..9).map(|key| Point::new(5 + key % 3 * 4, 14 + key / 3 * 4));

        for corner in keys {
            display
                .fill_solid(&Rectangle::new(corner, Size::new(3, 3)), BinaryColor::On)
                .unwrap();
        }
    }
}

impl Mode for Boot {
    async fn task(&mut self) {
        let mut display = DisplayProxy::new();

        for frame in 0..=FRAMES {
            display.idle().await;
            display.clear(BinaryColor::Off).unwrap();

            Self::draw_icon(&mut display);

            // The title slides in from the right edge during the first half, then the subtitle appears.
            let slide = (FRAMES / 2 - frame).max(0) * (DisplayProxy::WIDTH - TITLE_LEFT) / (FRAMES / 2);
            let progress = (DisplayProxy::WIDTH - TITLE_LEFT) * frame / FRAMES;

            Text::with_baseline(
                "NumCal",
                Point::new(TITLE_LEFT + slide, 0),
                MonoTextStyle::new(&FONT_10X20, BinaryColor::On),
                Baseline::Top,
            )
            .draw(&mut display)
            .unwrap();

            if slide == 0 {
                Text::with_baseline(
                    "numpad + calc",
                    Point::new(TITLE_LEFT, 17),
                    MonoTextStyle::new(&FONT_6X10, BinaryColor::On),
                    Baseline::Top,
                )
                .draw(&mut display)
                .unwrap();
            }

            display
                .fill_solid(
                    &Rectangle::new(Point::new(TITLE_LEFT, 29), Size::new(progress.unsigned_abs(), 2)),
                    BinaryColor::On,
                )
                .unwrap();
            display.flush().unwrap();

            Timer::after_millis(100).await;
        }
    }
}
