use crate::modes::{Mode, MODE_RUNNING};
use crate::tasks::{LED_STATE, HID_CHANNEL, HidEvent, KEYPAD_CHANNEL, Key, KeyboardLed, DisplayProxy};
use portable_atomic::{Ordering};
use embassy_sync::pubsub::{WaitResult};
use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::geometry::{Point, Size};
use embedded_graphics::pixelcolor::BinaryColor;

pub struct NumpadMode {
}

impl NumpadMode {
    pub fn new() -> Self {
        Self {
        }
    }

    fn draw_leds(&self, display: &mut DisplayProxy) {
        const LED_BOX_HEIGHT: u16 = 10;
        const LED_BOX_WIDTH: u16 = 20;
        const LED_BOX_SPACING: i32 = 5;
        const LED_BOX_THICKNESS: u32 = 2;
        const LED_VALUES: [KeyboardLed; 3] = [KeyboardLed::NumLock, KeyboardLed::CapsLock, KeyboardLed::ScrollLock];

        display.clear(BinaryColor::Off).unwrap();

        for (i, led) in LED_VALUES.iter().enumerate() {
            let offset_x = LED_BOX_SPACING + (i as i32 * (LED_BOX_WIDTH as i32 + LED_BOX_SPACING));

            display.fill_solid(
                &embedded_graphics::primitives::Rectangle::new(
                    Point::new(offset_x, 60),
                    Size::new(LED_BOX_WIDTH as u32, LED_BOX_HEIGHT as u32),
                ),
                BinaryColor::On,
            ).unwrap();

            if !LED_STATE.test(*led) {
                display.fill_solid(
                    &embedded_graphics::primitives::Rectangle::new(
                        Point::new(offset_x + LED_BOX_THICKNESS as i32, 21),
                        Size::new(LED_BOX_WIDTH as u32 - LED_BOX_THICKNESS * 2, LED_BOX_HEIGHT as u32 - LED_BOX_THICKNESS * 2),
                    ),
                    BinaryColor::Off,
                ).unwrap();
            }
        }

        display.flush().unwrap();
    }
}

impl Mode for NumpadMode {
    async fn task(&mut self) {
        let mut keypad = KEYPAD_CHANNEL.subscriber().unwrap();
        let mut display = DisplayProxy::new();
        let hid = HID_CHANNEL.sender();

        hid.send(HidEvent::Reset).await;

        while MODE_RUNNING.load(Ordering::Relaxed) {
            if let WaitResult::Message(event) = keypad.next_message().await {
                if matches!(event.key, Key::F1 | Key::F2 | Key::F3 | Key::F4) {
                    continue;
                }

                if let Some(keycode) = event.key.into_hid_keycode() {
                    if event.pressed {
                        hid.send(HidEvent::SetKey(keycode)).await;
                    } else {
                        hid.send(HidEvent::ReleaseKey(keycode)).await;
                    }
                }
            }

            self.draw_leds(&mut display);
        }
    }
}
