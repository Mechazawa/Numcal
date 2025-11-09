#![no_std]
#![no_main]
mod tasks;
mod utils;

use cortex_m::Peripherals;
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
use crate::tasks::{init_keypad, DisplayProxy};

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

    show_text("Boot Keyboard");

    init_keypad(&spawner, [
        peripherals.PIN_9.into(),
        peripherals.PIN_8.into(),
        peripherals.PIN_7.into(),
        peripherals.PIN_6.into(),
        peripherals.PIN_5.into(),
        peripherals.PIN_4.into(),
    ], [
        peripherals.PIN_26.into(),
        peripherals.PIN_27.into(),
        peripherals.PIN_28.into(),
        peripherals.PIN_29.into(),
    ]).await;

    show_text("Ready");

    // Wait for USB to enumerate and logger to be ready
    // todo add this to the init_usb with a timeout
    Timer::after_secs(2).await;

    // Draw text on display
    show_text("Waiting...");

    Timer::after_secs(9).await;

    show_text("Reboot");
    Timer::after_secs(1).await;

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

fn show_text(text: &str) {
    let mut display = DisplayProxy::new();

    display.clear(BinaryColor::Off).unwrap();

    let text_style = MonoTextStyleBuilder::new()
        .font(&FONT_6X10)
        .text_color(BinaryColor::On)
        .build();

    display.clear(BinaryColor::Off).unwrap();

    Text::with_baseline(text, Point::new(5, 38), text_style, Baseline::Middle)
        .draw(&mut display)
        .unwrap();

    display.flush().unwrap();
}