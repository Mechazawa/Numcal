use core::fmt::{self, Write};

const LETTERS: &str = "abcdefghijklmnopqrstuvwxyz";
const DIGITS: [&str; 10] = [
    "digit0", "digit1", "digit2", "digit3", "digit4", "digit5", "digit6", "digit7", "digit8", "digit9",
];

/// One key press as Ghostty's `send key` script command names it, with optional modifiers.
#[derive(Clone, Copy, Debug)]
pub struct Keystroke {
    modifiers: Option<&'static str>,
    name: &'static str,
}

impl Keystroke {
    pub const ENTER: Self = Self::key("enter");
    pub const ESCAPE: Self = Self::key("escape");
    pub const TAB: Self = Self::key("tab");
    pub const RIGHT: Self = Self::key("arrowRight");
    pub const LEFT: Self = Self::key("arrowLeft");
    pub const DOWN: Self = Self::key("arrowDown");
    pub const UP: Self = Self::key("arrowUp");

    pub const fn key(name: &'static str) -> Self {
        Self { modifiers: None, name }
    }

    /// `modifiers` is Ghostty's comma-separated list, such as `control` or `control,shift`.
    pub const fn with(modifiers: &'static str, name: &'static str) -> Self {
        Self {
            modifiers: Some(modifiers),
            name,
        }
    }

    /// Covers the characters the macros type: lowercase letters, digits, `/`, `-` and space.
    pub fn from_char(character: char) -> Option<Self> {
        let byte = u8::try_from(character).ok()?;

        let name = match byte {
            b'a'..=b'z' => {
                let index = usize::from(byte - b'a');

                &LETTERS[index..=index]
            }
            b'0'..=b'9' => DIGITS[usize::from(byte - b'0')],
            b' ' => "space",
            b'-' => "minus",
            b'/' => "slash",
            _ => return None,
        };

        Some(Self::key(name))
    }

    /// Writes the key as `name`, or `modifiers:name`.
    pub fn write_ghostty(self, out: &mut impl Write) -> fmt::Result {
        match self.modifiers {
            Some(modifiers) => write!(out, "{modifiers}:{}", self.name),
            None => write!(out, "{}", self.name),
        }
    }
}
