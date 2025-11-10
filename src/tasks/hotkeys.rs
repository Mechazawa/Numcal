use embassy_executor::Spawner;
use embassy_rp::rom_data::reset_to_usb_boot;
use embassy_sync::pubsub::WaitResult;
use embassy_time::Timer;
use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::Drawable;
use embedded_graphics::geometry::Point;
use embedded_graphics::mono_font::ascii::FONT_6X10;
use embedded_graphics::mono_font::MonoTextStyleBuilder;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::text::{Baseline, Text};
use crate::tasks::{key_pressed, DisplayProxy, Key, KEYPAD_CHANNEL};

pub async fn init(spawner: &Spawner) {
    spawner.spawn(reboot_hotkey_task().unwrap());
}

#[embassy_executor::task]
pub async fn reboot_hotkey_task() {
    let mut receiver = KEYPAD_CHANNEL.subscriber().unwrap();
    const REBOOT_KEYS: [Key; 4] = [
        Key::F1,
        Key::F2,
        Key::F3,
        Key::F4,
    ];

    loop {
        if let WaitResult::Message(event) = receiver.next_message().await {
            if !event.pressed {
                continue;
            }

            if !REBOOT_KEYS.iter().any(|key| event.key == *key) {
                continue;
            }

            if REBOOT_KEYS.iter().all(|key| key_pressed(key.clone())) {
                break;
            }
        }
    }

    let mut display = DisplayProxy::new();

    let text_style = MonoTextStyleBuilder::new()
        .font(&FONT_6X10)
        .text_color(BinaryColor::On)
        .build();

    display.clear(BinaryColor::Off).unwrap();

    Text::with_baseline("Reboot into BOOTSEL", Point::new(5, 38), text_style, Baseline::Middle)
        .draw(&mut display)
        .unwrap();

    display.flush().unwrap();

    Timer::after_millis(300).await;

    reset_to_usb_boot(0, 0);
}