//! Development-only serial commands (cargo feature `debug-link`):
//! `dump` logs the framebuffer as `FB <row> <hex>` lines, `key|down|up <Key>` injects key events.

use core::cell::RefCell;
use core::convert::Infallible;
use core::fmt::Write;
use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::Timer;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use heapless::String;

use crate::tasks::{DisplayAction, Key, inject_key};

const WIDTH: u32 = 132;
const HEIGHT: u32 = 64;
const ROW_BYTES: usize = WIDTH.div_ceil(8) as usize;

pub enum DebugCommand {
    Dump,
    Key(Key, Option<bool>),
}

impl DebugCommand {
    pub fn parse(line: &str) -> Option<Self> {
        let mut words = line.split(' ');
        let command = words.next()?;

        if command == "dump" {
            return Some(Self::Dump);
        }

        let pressed = match command {
            "key" => None,
            "down" => Some(true),
            "up" => Some(false),
            _ => return None,
        };

        let key = match words.next()? {
            "F1" => Key::F1,
            "F2" => Key::F2,
            "F3" => Key::F3,
            "F4" => Key::F4,
            "Lock" => Key::Lock,
            "Div" => Key::Div,
            "Mul" => Key::Mul,
            "Sub" => Key::Sub,
            "Add" => Key::Add,
            "Enter" => Key::Enter,
            "Dot" => Key::Dot,
            "0" => Key::D0,
            "1" => Key::D1,
            "2" => Key::D2,
            "3" => Key::D3,
            "4" => Key::D4,
            "5" => Key::D5,
            "6" => Key::D6,
            "7" => Key::D7,
            "8" => Key::D8,
            "9" => Key::D9,
            _ => return None,
        };

        Some(Self::Key(key, pressed))
    }
}

pub static DEBUG_COMMANDS: Channel<CriticalSectionRawMutex, DebugCommand, 8> = Channel::new();

/// Mirror of the display buffer, one bit per pixel, row-major.
struct Shadow([[u8; ROW_BYTES]; HEIGHT as usize]);

impl OriginDimensions for Shadow {
    fn size(&self) -> Size {
        Size::new(WIDTH, HEIGHT)
    }
}

impl DrawTarget for Shadow {
    type Color = BinaryColor;
    type Error = Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        pixels
            .into_iter()
            .filter_map(|Pixel(point, color)| {
                let x = u32::try_from(point.x).ok().filter(|&x| x < WIDTH)? as usize;
                let y = u32::try_from(point.y).ok().filter(|&y| y < HEIGHT)? as usize;

                Some((x, y, color))
            })
            .for_each(|(x, y, color)| match color {
                BinaryColor::On => self.0[y][x / 8] |= 0x80 >> (x % 8),
                BinaryColor::Off => self.0[y][x / 8] &= !(0x80 >> (x % 8)),
            });

        Ok(())
    }
}

static SHADOW: Mutex<CriticalSectionRawMutex, RefCell<Shadow>> =
    Mutex::new(RefCell::new(Shadow([[0; ROW_BYTES]; HEIGHT as usize])));

pub fn mirror(action: &DisplayAction) {
    SHADOW.lock(|shadow| {
        let mut shadow = shadow.borrow_mut();

        let _ = match action {
            DisplayAction::Clear(color) => shadow.clear(*color),
            DisplayAction::FillSolid(area, color) => shadow.fill_solid(area, *color),
            DisplayAction::Draw(pixels) => shadow.draw_iter(pixels.iter().copied()),
            DisplayAction::Flush => Ok(()),
        };
    });
}

pub fn init(spawner: Spawner) {
    spawner.spawn(debug_task().unwrap());
}

#[embassy_executor::task]
async fn debug_task() {
    loop {
        match DEBUG_COMMANDS.receive().await {
            DebugCommand::Dump => {
                for row in 0..HEIGHT as usize {
                    let bytes = SHADOW.lock(|shadow| shadow.borrow().0[row]);
                    let mut line: String<{ ROW_BYTES * 2 + 8 }> = String::new();
                    let _ = write!(line, "FB {row:02} ");

                    for byte in bytes {
                        let _ = write!(line, "{byte:02x}");
                    }

                    log::info!("{line}");
                    Timer::after_millis(4).await;
                }

                log::info!("FB END");
            }
            DebugCommand::Key(key, Some(pressed)) => inject_key(key, pressed),
            DebugCommand::Key(key, None) => {
                inject_key(key, true);
                Timer::after_millis(30).await;
                inject_key(key, false);
            }
        }
    }
}
