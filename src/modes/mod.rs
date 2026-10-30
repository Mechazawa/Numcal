use crate::modes::boot::Boot;
use crate::modes::calculator::Calculator;
use crate::modes::canvas::Canvas;
use crate::modes::claude::Claude;
use crate::modes::numpad::Numpad;
use crate::tasks::{KEYPAD_CHANNEL, Key, key_pressed};
use embassy_executor::Spawner;
use embassy_futures::select::select;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::pubsub::WaitResult;
use embassy_sync::signal::Signal;
use enum_dispatch::enum_dispatch;
use portable_atomic::{AtomicBool, AtomicU8, Ordering};

mod boot;
mod calculator;
mod canvas;
mod claude;
mod numpad;

#[enum_dispatch]
enum CurrentMode {
    Boot,
    Numpad,     // F1
    Calculator, // F2
    Claude,     // F3
    Canvas,     // F4
}

static MODE_RUNNING: AtomicBool = AtomicBool::new(true);
static TARGET_MODE: AtomicU8 = AtomicU8::new(0);
static MODE_SWITCH: Signal<CriticalSectionRawMutex, ()> = Signal::new();

const CANVAS_MODE: u8 = 3;

pub use canvas::{CANVAS, CANVAS_CHANGED};

/// Ends the running mode right away, also when it waits on something other than a key press.
fn switch_to(mode: u8) {
    if TARGET_MODE.swap(mode, Ordering::Relaxed) != mode {
        MODE_RUNNING.store(false, Ordering::Relaxed);
        MODE_SWITCH.signal(());
    }
}

pub fn show_canvas() {
    switch_to(CANVAS_MODE);
}

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

            match event.key {
                Key::F1 => switch_to(0),
                Key::F2 => switch_to(1),
                Key::F3 => switch_to(2),
                Key::F4 => switch_to(CANVAS_MODE),
                _ => {}
            }
        }
    }
}

#[embassy_executor::task]
async fn mode_handler_task() {
    let mut mode: CurrentMode = Boot::new().into();

    loop {
        MODE_RUNNING.store(true, Ordering::Relaxed);
        MODE_SWITCH.reset();
        select(mode.task(), MODE_SWITCH.wait()).await;

        mode = match TARGET_MODE.load(Ordering::Relaxed) {
            0 => Numpad::new().into(),
            1 => Calculator::new().into(),
            2 => Claude::new().into(),
            CANVAS_MODE => Canvas::new().into(),
            _ => mode,
        };
    }
}
