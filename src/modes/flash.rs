use embassy_time::Timer;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::DrawTarget;
use crate::modes::{Mode, MODE_RUNNING};
use crate::tasks::DisplayProxy;
use portable_atomic::{Ordering};
use crate::show_text;

pub struct FlashTest{}

impl FlashTest {
    pub fn new() -> Self {
        Self {}
    }
}

impl Mode for FlashTest {
    async fn task(&mut self) {
        let mut display = DisplayProxy::new();

        show_text("Flash test");

        Timer::after_secs(1).await;

        let mut inverted = false;

        while MODE_RUNNING.load(Ordering::Relaxed) {
            display.clear(if inverted {BinaryColor::On} else {BinaryColor::Off}).unwrap();
            display.flush().unwrap();

            Timer::after_secs(1).await;
            inverted = !inverted;
        }
    }
}