use core::cell::RefCell;
use core::fmt::Write;
use embassy_futures::select::{Either, select};
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::pubsub::WaitResult;
use embassy_sync::signal::Signal;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::mono_font::ascii::{FONT_4X6, FONT_6X10, FONT_10X20};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Circle, Line, PrimitiveStyle, Rectangle};
use embedded_graphics::text::{Baseline, Text};
use heapless::{String, Vec};
use portable_atomic::Ordering;

use crate::modes::{MODE_RUNNING, Mode};
use crate::tasks::{DisplayProxy, HID_CHANNEL, HOST_REQUESTS, HidEvent, KEYPAD_CHANNEL, Key, key_pressed};
use crate::utils::framebuffer::Framebuffer;

const MAX_OPS: usize = 40;

#[derive(Clone, Copy, Debug)]
enum Font {
    Tiny,
    Small,
    Big,
}

impl Font {
    fn style(self) -> MonoTextStyle<'static, BinaryColor> {
        match self {
            Font::Tiny => MonoTextStyle::new(&FONT_4X6, BinaryColor::On),
            Font::Small => MonoTextStyle::new(&FONT_6X10, BinaryColor::On),
            Font::Big => MonoTextStyle::new(&FONT_10X20, BinaryColor::On),
        }
    }
}

/// One drawing operation an agent sends as `canvas\t<op>`; see the numcal-display skill for the syntax.
#[derive(Clone, Debug)]
enum CanvasOp {
    Clear,
    Text {
        at: Point,
        font: Font,
        text: String<48>,
    },
    Rect {
        area: Rectangle,
        style: PrimitiveStyle<BinaryColor>,
    },
    Line {
        from: Point,
        to: Point,
    },
    Circle {
        at: Point,
        diameter: u32,
        style: PrimitiveStyle<BinaryColor>,
    },
    Invert(Rectangle),
    Bits {
        at: Point,
        width: u32,
        bytes: Vec<u8, 64>,
    },
}

impl CanvasOp {
    fn parse(text: &str) -> Option<Self> {
        let (name, arguments) = text.split_once(' ').unwrap_or((text, ""));
        let mut words = arguments.split(' ');
        let mut number = || words.next()?.parse::<i32>().ok();
        let style = |word: Option<&str>| match word? {
            "fill" => Some(PrimitiveStyle::with_fill(BinaryColor::On)),
            "line" => Some(PrimitiveStyle::with_stroke(BinaryColor::On, 1)),
            _ => None,
        };

        let op = match name {
            "clear" => Self::Clear,
            "text" => {
                let mut parts = arguments.splitn(4, ' ');
                let at = Point::new(parts.next()?.parse().ok()?, parts.next()?.parse().ok()?);
                let font = match parts.next()? {
                    "tiny" => Font::Tiny,
                    "small" => Font::Small,
                    "big" => Font::Big,
                    _ => return None,
                };

                Self::Text {
                    at,
                    font,
                    text: parts.next()?.chars().filter(char::is_ascii).take(48).collect(),
                }
            }
            "rect" => {
                let corner = Point::new(number()?, number()?);
                let size = Size::new(number()?.unsigned_abs(), number()?.unsigned_abs());

                Self::Rect {
                    area: Rectangle::new(corner, size),
                    style: style(words.next())?,
                }
            }
            "line" => Self::Line {
                from: Point::new(number()?, number()?),
                to: Point::new(number()?, number()?),
            },
            "circle" => {
                let at = Point::new(number()?, number()?);
                let diameter = number()?.unsigned_abs();

                Self::Circle {
                    at,
                    diameter,
                    style: style(words.next())?,
                }
            }
            "invert" => {
                let corner = Point::new(number()?, number()?);

                Self::Invert(Rectangle::new(
                    corner,
                    Size::new(number()?.unsigned_abs(), number()?.unsigned_abs()),
                ))
            }
            "bits" => {
                let at = Point::new(number()?, number()?);
                let width = number()?.unsigned_abs();
                let hex = words.next()?.as_bytes();

                Self::Bits {
                    at,
                    width,
                    bytes: hex
                        .chunks(2)
                        .map(|pair| u8::from_str_radix(core::str::from_utf8(pair).ok()?, 16).ok())
                        .collect::<Option<_>>()?,
                }
            }
            _ => return None,
        };

        Some(op)
    }

    fn render(&self, frame: &mut Framebuffer) {
        let _ = match self {
            Self::Clear => frame.clear(BinaryColor::Off),
            Self::Text { at, font, text } => Text::with_baseline(text, *at, font.style(), Baseline::Top)
                .draw(frame)
                .map(|_| ()),
            Self::Rect { area, style } => area.into_styled(*style).draw(frame),
            Self::Line { from, to } => Line::new(*from, *to)
                .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
                .draw(frame),
            Self::Circle { at, diameter, style } => Circle::new(*at, *diameter).into_styled(*style).draw(frame),
            Self::Invert(area) => {
                frame.invert(area);

                Ok(())
            }
            Self::Bits { at, width, bytes } => {
                let row_bytes = width.div_ceil(8).max(1);
                let lit = bytes.iter().zip(0u32..).flat_map(|(&byte, index)| {
                    (0..8u32)
                        .filter(move |bit| byte & (0x80 >> bit) != 0)
                        .map(move |bit| (index % row_bytes * 8 + bit, index / row_bytes))
                });

                frame.draw_iter(
                    lit.filter(|&(x, _)| x < *width)
                        .map(|(x, y)| Pixel(*at + Point::new(x.cast_signed(), y.cast_signed()), BinaryColor::On)),
                )
            }
        };
    }
}

/// The frame agents draw: ops collect in `pending` and become the shown `frame` on `flush`.
pub struct CanvasState {
    owner: String<16>,
    pending: Vec<CanvasOp, MAX_OPS>,
    frame: Vec<CanvasOp, MAX_OPS>,
}

impl CanvasState {
    const fn new() -> Self {
        Self {
            owner: String::new(),
            pending: Vec::new(),
            frame: Vec::new(),
        }
    }

    /// Applies one line sent as `canvas\t<line>`. Ops beyond [`MAX_OPS`] per frame are dropped.
    pub fn apply(&mut self, line: &str) {
        match line.split_once(' ').unwrap_or((line, "")) {
            ("owner", name) => self.owner = name.chars().filter(char::is_ascii).take(16).collect(),
            ("flush", _) => self.frame = core::mem::take(&mut self.pending),
            _ => {
                if let Some(op) = CanvasOp::parse(line) {
                    let _ = self.pending.push(op);
                }
            }
        }
    }
}

pub static CANVAS: Mutex<CriticalSectionRawMutex, RefCell<CanvasState>> = Mutex::new(RefCell::new(CanvasState::new()));
pub static CANVAS_CHANGED: Signal<CriticalSectionRawMutex, ()> = Signal::new();

pub struct Canvas {
    /// Whether a key was pressed while Lock was held, so releasing Lock is a mode chord, not a key.
    lock_chord: bool,
}

impl Canvas {
    pub fn new() -> Self {
        Self { lock_chord: false }
    }

    fn draw(display: &mut DisplayProxy) {
        let (owner, ops) = CANVAS.lock(|canvas| {
            let canvas = canvas.borrow();

            (canvas.owner.clone(), canvas.frame.clone())
        });
        let mut frame = Framebuffer::new();

        if ops.is_empty() {
            let small = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
            let tiny = MonoTextStyle::new(&FONT_4X6, BinaryColor::On);
            let waiting = if owner.is_empty() {
                "waiting for an agent"
            } else {
                owner.as_str()
            };

            let _ = Text::with_baseline("Canvas", Point::new(2, 4), small, Baseline::Top).draw(&mut frame);
            let _ = Text::with_baseline(waiting, Point::new(2, 18), tiny, Baseline::Top).draw(&mut frame);
        }

        for op in &ops {
            op.render(&mut frame);
        }

        display.clear(BinaryColor::Off).unwrap();
        display.draw_iter(frame.lit_pixels()).unwrap();
        display.flush().unwrap();
    }

    fn send_key(key: &str) {
        let mut request: String<128> = String::new();
        let _ = writeln!(request, "key\t{key}");
        let _ = HOST_REQUESTS.try_send(request);
    }
}

impl Mode for Canvas {
    async fn task(&mut self) {
        let mut display = DisplayProxy::new();
        let mut keypad = KEYPAD_CHANNEL.subscriber().unwrap();

        HID_CHANNEL.sender().send(HidEvent::Reset).await;

        while MODE_RUNNING.load(Ordering::Relaxed) {
            display.idle().await;
            Self::draw(&mut display);

            let Either::First(WaitResult::Message(event)) = select(keypad.next_message(), CANVAS_CHANGED.wait()).await
            else {
                continue;
            };

            match (event.key, event.pressed) {
                (Key::Lock, true) => self.lock_chord = false,
                (Key::Lock, false) if !self.lock_chord => Self::send_key("lock"),
                (_, true) if key_pressed(Key::Lock) => self.lock_chord = true,
                (key, true) => {
                    if let Some(name) = key.name() {
                        Self::send_key(name);
                    }
                }
                _ => {}
            }
        }
    }
}
