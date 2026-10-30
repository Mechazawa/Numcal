use crate::modes::{MODE_RUNNING, Mode};
use crate::tasks::{DisplayProxy, HID_CHANNEL, HidEvent, KEYPAD_CHANNEL, Key, KeyboardLed, LED_CHANGED, LED_STATE};
use embassy_futures::select::{Either, select};
use embassy_sync::pubsub::WaitResult;
use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::geometry::{Point, Size};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::primitives::Rectangle;
use portable_atomic::Ordering;

pub struct Numpad {}

impl Numpad {
    pub fn new() -> Self {
        Self {}
    }

    fn draw_leds(display: &mut DisplayProxy) {
        const LED_BOX_SIZE: Size = Size::new(20, 10);
        const LED_BOX_SPACING: i32 = 5;
        const LED_BOX_THICKNESS: i32 = 2;
        const LED_BOX_TOP: i32 = DisplayProxy::HEIGHT - LED_BOX_SIZE.height.cast_signed() - 4;
        const LED_VALUES: [KeyboardLed; 3] = [KeyboardLed::Num, KeyboardLed::Caps, KeyboardLed::Scroll];

        display.clear(BinaryColor::Off).unwrap();

        let lefts =
            (LED_BOX_SPACING..).step_by((LED_BOX_SIZE.width.cast_signed() + LED_BOX_SPACING).unsigned_abs() as usize);

        for (led, left) in LED_VALUES.into_iter().zip(lefts) {
            let outline = Rectangle::new(Point::new(left, LED_BOX_TOP), LED_BOX_SIZE);

            display.fill_solid(&outline, BinaryColor::On).unwrap();

            if !LED_STATE.test(led) {
                display
                    .fill_solid(&outline.offset(-LED_BOX_THICKNESS), BinaryColor::Off)
                    .unwrap();
            }
        }

        display.flush().unwrap();
    }
}

impl Mode for Numpad {
    async fn task(&mut self) {
        let mut keypad = KEYPAD_CHANNEL.subscriber().unwrap();
        let mut display = DisplayProxy::new();
        let hid = HID_CHANNEL.sender();

        hid.send(HidEvent::Reset).await;

        // Draw LED indicators immediately on mode entry
        Self::draw_leds(&mut display);

        while MODE_RUNNING.load(Ordering::Relaxed) {
            match select(keypad.next_message(), LED_CHANGED.wait()).await {
                Either::First(WaitResult::Message(event)) => {
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
                Either::First(_) => continue,
                Either::Second(()) => {}
            }

            Self::draw_leds(&mut display);
        }
    }
}
