use portable_atomic::{AtomicBool, Ordering};
use embassy_executor::Spawner;
use embassy_sync::pubsub::WaitResult;
use static_cell::StaticCell;
use crate::modes::numpad::NumpadMode;
use crate::tasks::{key_pressed, Key, KEYPAD_CHANNEL};

mod numpad;

enum CurrentMode {
    Numpad(NumpadMode), // F1
    // F2 (todo)
    // F3 (todo)
    // F4 (todo)
}

trait Mode {
    fn new() -> Self;

    async fn task(running: AtomicBool);
}

pub async fn init_mode_handler(spawner: &Spawner) {
    spawner.spawn(mode_handler_task().unwrap());
    spawner.spawn(mode_switcher_task().unwrap());
}

#[embassy_executor::task]
async fn mode_switcher_task() {
    let mut keypad_receiver = KEYPAD_CHANNEL.subscriber().unwrap();

    loop {
        if let WaitResult::Message(event) = keypad_receiver.next_message().await {
            if !event.pressed || !key_pressed(Key::Lock) {
                continue;
            }

            // if mode is not the correct mode switch it if one of F1, F2, F3 or F4 was just pressed
        }
    }
}

#[embassy_executor::task]
async fn mode_handler_task() {
    // this should be the task that runs the mode task
}