use crate::modes::{Mode, MODE_RUNNING};
use crate::show_text;
use crate::tasks::{HID_CHANNEL, HidEvent, KEYPAD_CHANNEL, Key};
use portable_atomic::{Ordering};
use embassy_sync::pubsub::{WaitResult};
use core::fmt::Write;

pub struct NumpadMode {
}

impl NumpadMode {
    pub fn new() -> Self {
        Self {
        }
    }
}

impl Mode for NumpadMode {
    async fn task(&mut self) {
        let mut keypad = KEYPAD_CHANNEL.subscriber().unwrap();
        let hid = HID_CHANNEL.sender();

        hid.send(HidEvent::Reset).await;

        while MODE_RUNNING.load(Ordering::Relaxed) {
            if let WaitResult::Message(event) = keypad.next_message().await {
                if match event.key {
                    Key::F1 | Key::F2 | Key::F3 | Key::F4 => true,
                    _ => false,
                } {
                    continue;
                }

                if let Some(keycode) = event.key.into_hid_keycode() {
                    if event.pressed {
                        hid.send(HidEvent::SetKey(keycode)).await;
                    } else {
                        hid.send(HidEvent::ReleaseKey(keycode)).await;
                    }
                }

                let mut str = heapless::String::<32>::new();

                if let Ok(()) = write!(
                    &mut str,
                    "{} {:?}",
                    if event.pressed { "PRESS " } else { "RELEASE " },
                    event.key
                ) {
                    show_text(&str);
                } else {
                    show_text("Whoops");
                }
            }
        }
    }
}
