//! Development-only serial command (cargo feature `debug-link`): `key|down|up <Key>` injects key events.

use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::Timer;

use crate::tasks::{Key, inject_key};

pub struct DebugCommand(Key, Option<bool>);

impl DebugCommand {
    pub fn parse(line: &str) -> Option<Self> {
        let mut words = line.split(' ');

        let pressed = match words.next()? {
            "key" => None,
            "down" => Some(true),
            "up" => Some(false),
            _ => return None,
        };

        let key = match words.next()? {
            "F1" => Key::F1,
            "F2" => Key::F2,
            "F3" => Key::F3,
            "F4" => Key::F4,
            "Lock" => Key::Lock,
            "Div" => Key::Div,
            "Mul" => Key::Mul,
            "Sub" => Key::Sub,
            "Add" => Key::Add,
            "Enter" => Key::Enter,
            "Dot" => Key::Dot,
            "0" => Key::D0,
            "1" => Key::D1,
            "2" => Key::D2,
            "3" => Key::D3,
            "4" => Key::D4,
            "5" => Key::D5,
            "6" => Key::D6,
            "7" => Key::D7,
            "8" => Key::D8,
            "9" => Key::D9,
            _ => return None,
        };

        Some(Self(key, pressed))
    }
}

pub static DEBUG_COMMANDS: Channel<CriticalSectionRawMutex, DebugCommand, 8> = Channel::new();

pub fn init(spawner: Spawner) {
    spawner.spawn(debug_task().unwrap());
}

#[embassy_executor::task]
async fn debug_task() {
    loop {
        match DEBUG_COMMANDS.receive().await {
            DebugCommand(key, Some(pressed)) => inject_key(key, pressed),
            DebugCommand(key, None) => {
                inject_key(key, true);
                Timer::after_millis(30).await;
                inject_key(key, false);
            }
        }
    }
}
