use core::fmt::{self, Write};

use crate::tasks::{HID_CHANNEL, HidEvent};

pub const CTRL: u8 = 0x01;
pub const SHIFT: u8 = 0x02;

const LETTERS: &str = "abcdefghijklmnopqrstuvwxyz";
const DIGITS: [&str; 10] = [
    "digit0", "digit1", "digit2", "digit3", "digit4", "digit5", "digit6", "digit7", "digit8", "digit9",
];

/// One key press, as a USB HID keycode and as the key name Ghostty's `send key` script command accepts.
#[derive(Clone, Copy, Debug)]
pub struct Keystroke {
    modifier: u8,
    keycode: u8,
    name: &'static str,
}

impl Keystroke {
    pub const ENTER: Self = Self::key(0x28, "enter");
    pub const ESCAPE: Self = Self::key(0x29, "escape");
    pub const TAB: Self = Self::key(0x2B, "tab");
    pub const RIGHT: Self = Self::key(0x4F, "arrowRight");
    pub const LEFT: Self = Self::key(0x50, "arrowLeft");
    pub const DOWN: Self = Self::key(0x51, "arrowDown");
    pub const UP: Self = Self::key(0x52, "arrowUp");

    pub const fn key(keycode: u8, name: &'static str) -> Self {
        Self {
            modifier: 0,
            keycode,
            name,
        }
    }

    pub const fn with(modifier: u8, keycode: u8, name: &'static str) -> Self {
        Self {
            modifier,
            keycode,
            name,
        }
    }

    /// Covers the characters the macros type: lowercase letters, digits, `/`, `-` and space.
    pub fn from_char(character: char) -> Option<Self> {
        let byte = u8::try_from(character).ok()?;

        let stroke = match byte {
            b'a'..=b'z' => {
                let index = usize::from(byte - b'a');

                Self::key(0x04 + byte - b'a', &LETTERS[index..=index])
            }
            b'1'..=b'9' => Self::key(0x1E + byte - b'1', DIGITS[usize::from(byte - b'0')]),
            b'0' => Self::key(0x27, DIGITS[0]),
            b' ' => Self::key(0x2C, "space"),
            b'-' => Self::key(0x2D, "minus"),
            b'/' => Self::key(0x38, "slash"),
            _ => return None,
        };

        Some(stroke)
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

    /// Writes the key as `name`, or `modifiers:name` with Ghostty's comma-separated modifier names.
    pub fn write_ghostty(self, out: &mut impl Write) -> fmt::Result {
        match self.modifier {
            0 => write!(out, "{}", self.name),
            CTRL => write!(out, "control:{}", self.name),
            SHIFT => write!(out, "shift:{}", self.name),
            _ => write!(out, "control,shift:{}", self.name),
        }
    }
}
