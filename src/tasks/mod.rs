mod claude_link;
mod display;
mod hotkeys;
mod keypad;
mod usb;
mod watchdog;

#[cfg(feature = "debug-link")]
pub use display::DisplayAction;
pub use display::DisplayProxy;
pub use display::init as init_display;
pub use hotkeys::init as init_hotkeys;
pub use keypad::init as init_keypad;
#[cfg(feature = "debug-link")]
pub use keypad::inject_key;
pub use keypad::{KEYPAD_CHANNEL, Key, key_pressed};
pub use usb::init as init_usb;
pub use usb::{HID_CHANNEL, HidEvent, KeyboardLed, LED_CHANGED, LED_STATE};
pub use watchdog::init as init_watchdog;

pub use claude_link::{Activity, CLAUDE_CHANGED, HOST_REQUESTS, Sessions, claude_sessions, helper_connected};
