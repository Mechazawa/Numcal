use core::fmt::Write;
use embassy_rp::gpio::Output;
use embassy_rp::spi::Spi;
use embassy_time::{Duration, Instant, Timer};
use embedded_graphics::mono_font::ascii::FONT_6X10;
use embedded_graphics::mono_font::MonoTextStyleBuilder;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::text::{Baseline, Text};
use embedded_hal_bus::spi::ExclusiveDevice;
use ssd1306::prelude::*;
use ssd1306::Ssd1306;

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
    defmt::info!("Display uptime counter task started");

    // Record boot time
    let boot_time = Instant::now();

    // Create text style
    let text_style = MonoTextStyleBuilder::new()
        .font(&FONT_6X10)
        .text_color(BinaryColor::On)
        .build();

    loop {
        // Calculate elapsed time since boot
        let elapsed = Instant::now() - boot_time;
        let total_seconds = elapsed.as_secs();

        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let seconds = total_seconds % 60;

        // Format as hh:mm:ss
        let mut time_str: heapless::String<16> = heapless::String::new();
        write!(&mut time_str, "{hours:02}:{minutes:02}:{seconds:02}").unwrap();

        defmt::trace!("Uptime: {}", time_str.as_str());

        // Clear display
        display.clear(BinaryColor::Off).unwrap();

        // Draw uptime text centered
        Text::with_baseline(
            time_str.as_str(),
            Point::new(40, 32),
            text_style,
            Baseline::Middle,
        )
        .draw(display)
        .unwrap();

        // Flush to display
        if let Ok(()) = display.flush() {
            defmt::trace!("Display updated successfully");
        } else {
            defmt::error!("Display flush failed!");
        }

        // Update every second
        Timer::after(Duration::from_secs(1)).await;
    }
}
