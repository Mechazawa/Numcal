#![no_std]
#![no_main]

mod tasks;

use embassy_executor::Spawner;
use embassy_time::Timer;
use embassy_rp::config::Config;
use embedded_graphics::Drawable;
use embedded_graphics::mono_font::ascii::FONT_6X10;
use embedded_graphics::mono_font::MonoTextStyleBuilder;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::{DrawTarget, Point, Primitive, Size};
use embedded_graphics::primitives::{PrimitiveStyle, Rectangle};
use embedded_graphics::text::{Baseline, Text};
use {defmt_rtt as _, panic_probe as _};
use log::info;

use tasks::init_usb;
use tasks::init_display;
use crate::tasks::DisplayProxy;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let peripherals = embassy_rp::init(Config::default());

    init_usb(
        &spawner,
        peripherals.USB
    ).await;

    init_display(
        &spawner,
        peripherals.SPI1,
        peripherals.PIN_14,
        peripherals.PIN_15,
        peripherals.PIN_13,
        peripherals.PIN_3,
        peripherals.PIN_10,
    ).await;

    // Wait for USB to enumerate and logger to be ready
    // todo add this to the init_usb with a timeout
    Timer::after_secs(2).await;

    // Draw text on display
    let mut display = DisplayProxy::new();

    display.clear(BinaryColor::Off).unwrap();

    let text_style = MonoTextStyleBuilder::new()
        .font(&FONT_6X10)
        .text_color(BinaryColor::On)
        .build();

    Rectangle::new(Point::new(124, 0), Size::new(4, 64))
        .into_styled(PrimitiveStyle::with_fill(BinaryColor::Off))
        .draw(&mut display)
        .unwrap();

    // Draw text
    Text::with_baseline("Test 123", Point::new(5, 38), text_style, Baseline::Middle)
        .draw(&mut display)
        .unwrap();

    display.flush().unwrap();

    // Wait a bit so the message can be seen
    Timer::after_secs(3).await;

    info!("Rebooting to BOOTSEL mode...");

    // Give time for the log message to be transmitted
    Timer::after_millis(100).await;

    // Reboot into bootsel mode
    embassy_rp::rom_data::reset_to_usb_boot(0, 0);

    // Should never reach here
    loop {
        Timer::after_secs(1).await;
    }
}
