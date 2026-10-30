#![no_std]
#![no_main]
#[cfg(feature = "debug-link")]
mod debug_link;
mod modes;
mod tasks;
mod utils;

use crate::modes::init_mode_handler;
use crate::tasks::{DisplayProxy, init_hotkeys, init_keypad, init_watchdog};
use core::panic::PanicInfo;
use embassy_executor::Spawner;
use embassy_rp::config::Config;
use embassy_rp::gpio::{Input, Level, Output, Pull};
use embassy_rp::rom_data::reset_to_usb_boot;
use embassy_time::Timer;
use embedded_graphics::Drawable;
use embedded_graphics::mono_font::MonoTextStyleBuilder;
use embedded_graphics::mono_font::ascii::FONT_6X10;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::{DrawTarget, Point};
use embedded_graphics::text::{Baseline, Text};
use tasks::init_display;
use tasks::init_usb;

/// Reboots into BOOTSEL, so a firmware that crashes during startup can still be flashed over USB.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    reset_to_usb_boot(0, 0);

    loop {
        cortex_m::asm::wfi();
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let mut peripherals = embassy_rp::init(Config::default());

    // Holding F1 while plugging in reboots into BOOTSEL before any other code can crash or hang.
    {
        let _f1_row = Output::new(peripherals.PIN_9.reborrow(), Level::Low);
        let f1_column = Input::new(peripherals.PIN_26.reborrow(), Pull::Up);

        Timer::after_micros(50).await;

        if f1_column.is_low() {
            reset_to_usb_boot(0, 0);
        }
    }

    init_watchdog(&spawner, peripherals.WATCHDOG).await;

    init_usb(&spawner, peripherals.USB).await;

    init_display(
        &spawner,
        peripherals.SPI1,
        peripherals.PIN_14,
        peripherals.PIN_15,
        peripherals.PIN_13,
        peripherals.PIN_3,
        peripherals.PIN_10,
    )
    .await;

    show_text("Boot Keyboard");

    init_keypad(
        &spawner,
        [
            peripherals.PIN_9.into(),
            peripherals.PIN_8.into(),
            peripherals.PIN_7.into(),
            peripherals.PIN_6.into(),
            peripherals.PIN_5.into(),
            peripherals.PIN_4.into(),
        ],
        [
            peripherals.PIN_26.into(),
            peripherals.PIN_27.into(),
            peripherals.PIN_28.into(),
            peripherals.PIN_29.into(),
        ],
    )
    .await;

    init_hotkeys(&spawner).await;
    init_mode_handler(&spawner).await;

    #[cfg(feature = "debug-link")]
    debug_link::init(spawner);

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

    Text::with_baseline(
        text,
        Point::new(2, DisplayProxy::HEIGHT / 2),
        text_style,
        Baseline::Middle,
    )
    .draw(&mut display)
    .unwrap();

    display.flush().unwrap();
}
