use crate::modes::Mode;
use crate::show_text;
use crate::tasks::{HID_CHANNEL, HidEvent, KEYPAD_CHANNEL, Key};
use portable_atomic::{AtomicBool, Ordering};
use embassy_sync::pubsub::WaitResult;

pub struct NumpadMode {}

impl Mode for NumpadMode {
    fn new() -> Self {
        Self {}
    }

    async fn task(running: AtomicBool) {
        let mut keypad_receiver = KEYPAD_CHANNEL.subscriber().unwrap();
        let hid_sender = HID_CHANNEL.sender();

        hid_sender.send(HidEvent::Reset).await;

        while running.load(Ordering::Relaxed) {
            if let WaitResult::Message(event) = keypad_receiver.next_message().await {
                if match event.key {
                    Key::F1 | Key::F2 | Key::F3 | Key::F4 => true,
                    _ => false,
                } {
                    continue;
                }

                if let Some(keycode) = event.key.into_hid_keycode() {
                    if event.pressed {
                        hid_sender.send(HidEvent::SetKey(keycode)).await;
                    } else {
                        hid_sender.send(HidEvent::ReleaseKey(keycode)).await;
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
