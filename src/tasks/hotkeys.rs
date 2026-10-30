use embassy_executor::Spawner;
use embassy_rp::rom_data::reset_to_usb_boot;
use embassy_sync::pubsub::WaitResult;
use embassy_time::Timer;
use crate::show_text;
use crate::tasks::{key_pressed, Key, KEYPAD_CHANNEL};

const REBOOT_KEYS: [Key; 4] = [
    Key::F1,
    Key::F2,
    Key::F3,
    Key::F4,
];

pub async fn init(spawner: &Spawner) {
    spawner.spawn(reboot_hotkey_task().unwrap());
}

#[embassy_executor::task]
pub async fn reboot_hotkey_task() {
    let mut receiver = KEYPAD_CHANNEL.subscriber().unwrap();

    loop {
        if let WaitResult::Message(event) = receiver.next_message().await {
            if !event.pressed {
                continue;
            }

            if !REBOOT_KEYS.contains(&event.key) {
                continue;
            }

            if REBOOT_KEYS.iter().all(|key| key_pressed(*key)) {
                break;
            }
        }
    }

    show_text("Reboot into BOOTSEL");

    Timer::after_millis(300).await;

    reset_to_usb_boot(0, 0);
}