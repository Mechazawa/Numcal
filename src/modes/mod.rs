use crate::modes::boot::Boot;
use crate::modes::calculator::Calculator;
use crate::modes::claude::Claude;
use crate::modes::flash::FlashTest;
use crate::modes::numpad::Numpad;
use crate::tasks::{KEYPAD_CHANNEL, Key, key_pressed};
use embassy_executor::Spawner;
use embassy_sync::pubsub::WaitResult;
use enum_dispatch::enum_dispatch;
use portable_atomic::{AtomicBool, AtomicU8, Ordering};

mod boot;
mod calculator;
mod claude;
mod flash;
mod numpad;

#[enum_dispatch]
enum CurrentMode {
    Boot,
    Numpad,     // F1
    Calculator, // F2
    Claude,     // F3
    FlashTest,  // F4
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
    let mut mode: CurrentMode = Boot::new().into();

    loop {
        MODE_RUNNING.store(true, Ordering::Relaxed);
        mode.task().await;

        mode = match TARGET_MODE.load(Ordering::Relaxed) {
            0 => Numpad::new().into(),
            1 => Calculator::new().into(),
            2 => Claude::new().into(),
            3 => FlashTest::new().into(),
            _ => mode,
        };
    }
}
