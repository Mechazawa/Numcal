use embassy_time::Timer;
use crate::modes::Mode;
use crate::show_text;

pub struct Boot{}

impl Boot {
    pub fn new() -> Self {
        Self {}
    }
}

impl Mode for Boot {
    async fn task(&mut self) {
        show_text("NumCal");

        Timer::after_secs(2).await;
    }
}
