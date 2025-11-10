#![no_std]
#![no_main]
mod tasks;
mod utils;
mod modes;

use embassy_executor::Spawner;
use embassy_rp::config::Config;
use embedded_graphics::Drawable;
use embedded_graphics::mono_font::ascii::FONT_6X10;
use embedded_graphics::mono_font::MonoTextStyleBuilder;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::{DrawTarget, Point};
use embedded_graphics::text::{Baseline, Text};
use embassy_time::Timer;
use tasks::init_usb;
use tasks::init_display;
use crate::modes::init_mode_handler;
use crate::tasks::{init_hotkeys, init_keypad, DisplayProxy};

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

    init_hotkeys(&spawner).await;
    init_mode_handler(&spawner).await;

    show_text("Ready");

    // Busy loop
    loop {
        Timer::after_secs(1).await;
    }
}

fn show_text(text: &str) {
    let mut display = DisplayProxy::new();

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