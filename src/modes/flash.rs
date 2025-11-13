use crate::modes::{Mode, MODE_RUNNING};
use crate::tasks::DisplayProxy;
use portable_atomic::Ordering;
use embassy_time::{Duration, Timer};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;

const CHECKERBOARD_SIZE: u32 = 8; // Size of each square in the checkerboard
const FLASH_INTERVAL: Duration = Duration::from_millis(500); // Flash every 500ms

pub struct FlashMode {
    inverted: bool,
}

impl FlashMode {
    pub fn new() -> Self {
        Self {
            inverted: false,
        }
    }

    fn draw_checkerboard(&self, display: &mut DisplayProxy, inverted: bool) {
        display.clear(BinaryColor::Off).unwrap();

        let width = 128;
        let height = 64;

        // Draw checkerboard pattern
        for y in (0..height).step_by(CHECKERBOARD_SIZE as usize) {
            for x in (0..width).step_by(CHECKERBOARD_SIZE as usize) {
                let square_x = x / CHECKERBOARD_SIZE;
                let square_y = y / CHECKERBOARD_SIZE;

                // Determine if this square should be filled
                let should_fill = (square_x + square_y) % 2 == 0;
                let fill = if inverted { !should_fill } else { should_fill };

                if fill {
                    let rect = Rectangle::new(
                        Point::new(x as i32, y as i32),
                        Size::new(CHECKERBOARD_SIZE, CHECKERBOARD_SIZE)
                    );
                    display.fill_solid(&rect, BinaryColor::On).unwrap();
                }
            }
        }

        display.flush().unwrap();
    }
}

impl Mode for FlashMode {
    async fn task(&mut self) {
        let mut display = DisplayProxy::new();

        while MODE_RUNNING.load(Ordering::Relaxed) {
            self.draw_checkerboard(&mut display, self.inverted);
            self.inverted = !self.inverted;

            Timer::after(FLASH_INTERVAL).await;
        }
    }
}
