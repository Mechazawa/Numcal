mod usb;
mod display;
mod keypad;
mod hotkeys;
mod watchdog;

pub use display::init as init_display;
pub use display::DisplayProxy;
pub use hotkeys::init as init_hotkeys;
pub use keypad::init as init_keypad;
pub use keypad::{key_pressed, Key, KEYPAD_CHANNEL};
pub use usb::init as init_usb;
pub use usb::{HidEvent, HID_CHANNEL};
pub use watchdog::init as init_watchdog;
