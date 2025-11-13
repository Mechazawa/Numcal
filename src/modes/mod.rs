use portable_atomic::{AtomicBool, AtomicU8, Ordering};
use embassy_executor::Spawner;
use embassy_sync::pubsub::WaitResult;
use enum_dispatch::enum_dispatch;
use crate::modes::boot::BootMode;
use crate::modes::numpad::NumpadMode;
use crate::modes::flash::FlashMode;
use crate::tasks::{key_pressed, Key, KEYPAD_CHANNEL};

mod numpad;
mod boot;
mod flash;

#[enum_dispatch]
enum CurrentMode {
    BootMode,
    NumpadMode, // F1
    // F2 (todo)
    // F3 (todo)
    FlashMode, // F4
}

static MODE_RUNNING: AtomicBool = AtomicBool::new(true);
static TARGET_MODE: AtomicU8 = AtomicU8::new(0);

#[enum_dispatch(CurrentMode)]
trait Mode {
    async fn task(&mut self);
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

            let next_mode = match event.key {
                Key::F1 => 0,
                Key::F2 => 1,
                Key::F3 => 2,
                Key::F4 => 3,
                _ => continue,
            };

            if TARGET_MODE.load(Ordering::Relaxed) != next_mode {
                TARGET_MODE.store(next_mode, Ordering::Relaxed);
                MODE_RUNNING.store(false, Ordering::Relaxed);
            }
        }
    }
}

#[embassy_executor::task]
async fn mode_handler_task() {
    // this should be the task that runs the mode task
    let mut mode: CurrentMode = BootMode::new().into();

    loop {
        MODE_RUNNING.store(true, Ordering::Relaxed);
        mode.task().await;

        mode = match TARGET_MODE.load(Ordering::Relaxed) {
            0 => NumpadMode::new().into(),
            3 => FlashMode::new().into(),
            _ => mode,
        };
    }
}