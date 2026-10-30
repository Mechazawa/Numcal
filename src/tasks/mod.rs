mod usb;
mod display;
mod keypad;
mod hotkeys;
mod watchdog;
mod claude_link;

pub use display::init as init_display;
pub use display::DisplayProxy;
#[cfg(feature = "debug-link")]
pub use display::DisplayAction;
#[cfg(feature = "debug-link")]
pub use keypad::inject_key;
pub use hotkeys::init as init_hotkeys;
pub use keypad::init as init_keypad;
pub use keypad::{key_pressed, Key, KEYPAD_CHANNEL};
pub use usb::init as init_usb;
pub use usb::{HidEvent, HID_CHANNEL, LED_STATE, LED_CHANGED, KeyboardLed};
pub use watchdog::init as init_watchdog;

pub use claude_link::{claude_sessions, Activity, Sessions, CLAUDE_CHANGED};
