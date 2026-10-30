use crate::tasks::{HidEvent, HID_CHANNEL};

pub const CTRL: u8 = 0x01;
pub const SHIFT: u8 = 0x02;

#[derive(Clone, Copy, Debug)]
pub struct Keystroke {
    modifier: u8,
    keycode: u8,
}

impl Keystroke {
    pub const ENTER: Self = Self::key(0x28);
    pub const ESCAPE: Self = Self::key(0x29);
    pub const TAB: Self = Self::key(0x2B);
    pub const RIGHT: Self = Self::key(0x4F);
    pub const LEFT: Self = Self::key(0x50);
    pub const DOWN: Self = Self::key(0x51);
    pub const UP: Self = Self::key(0x52);

    pub const fn key(keycode: u8) -> Self {
        Self { modifier: 0, keycode }
    }

    pub const fn with(modifier: u8, keycode: u8) -> Self {
        Self { modifier, keycode }
    }

    /// Covers the characters the macros type: lowercase letters, digits, `/`, `-` and space.
    pub fn from_char(character: char) -> Option<Self> {
        let byte = u8::try_from(character).ok()?;

        let keycode = match byte {
            b'a'..=b'z' => 0x04 + byte - b'a',
            b'1'..=b'9' => 0x1E + byte - b'1',
            b'0' => 0x27,
            b' ' => 0x2C,
            b'-' => 0x2D,
            b'/' => 0x38,
            _ => return None,
        };

        Some(Self::key(keycode))
    }

    pub async fn send(self) {
        let hid = HID_CHANNEL.sender();

        if self.modifier != 0 {
            hid.send(HidEvent::SetModifier(self.modifier)).await;
        }

        hid.send(HidEvent::SetKey(self.keycode)).await;
        hid.send(HidEvent::ReleaseKey(self.keycode)).await;

        if self.modifier != 0 {
            hid.send(HidEvent::ReleaseModifier(self.modifier)).await;
        }
    }
}
