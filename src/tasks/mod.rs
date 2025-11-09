use embassy_sync::channel::Channel;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;

mod usb;
mod display;

// Todo have things that implement Drawable instead of strings
pub static DISPLAY_CHANNEL: Channel<ThreadModeRawMutex, heapless::String<64>, 2> = Channel::new();

pub use usb::init as init_usb;
pub use display::init as init_display;