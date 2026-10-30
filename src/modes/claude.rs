use core::fmt::Write;
use embassy_futures::select::{select3, Either3};
use embassy_sync::pubsub::WaitResult;
use embassy_time::{Duration, Instant, Timer};
use embedded_graphics::mono_font::ascii::FONT_6X10;
use embedded_graphics::mono_font::{MonoTextStyle, MonoTextStyleBuilder};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Line, PrimitiveStyle, Rectangle};
use embedded_graphics::text::{Baseline, Text};
use heapless::String;
use portable_atomic::Ordering;

use crate::modes::{Mode, MODE_RUNNING};
use crate::tasks::{
    claude_sessions, key_pressed, Activity, DisplayProxy, HidEvent, Key, CLAUDE_CHANGED, HID_CHANNEL,
    KEYPAD_CHANNEL, Sessions,
};
use crate::utils::keystroke::{Keystroke, CTRL, SHIFT};

const FRAME: Duration = Duration::from_millis(125);
const FEEDBACK_TIME: Duration = Duration::from_millis(1200);
const TITLE: &str = "Claude Code";
/// Text columns that fit beside the mascot and left of the session grid.
const TEXT_COLUMNS: usize = 13;
const VERBS: [&str; 12] = [
    "Clauding",
    "Pondering",
    "Noodling",
    "Percolating",
    "Cogitating",
    "Ruminating",
    "Brewing",
    "Simmering",
    "Musing",
    "Conjuring",
    "Tinkering",
    "Scheming",
];

const CLAWD_COLS: u32 = 18;
const CLAWD: [u32; 5] = [
    sprite_row(b"   ############   "),
    sprite_row(b"   ## ###### ##   "),
    sprite_row(b" ################ "),
    sprite_row(b"   ############   "),
    sprite_row(b"    # #    # #    "),
];
const EYES: u32 = (1 << 5) | (1 << 12);
const ARMS: u32 = (1 << 1) | (1 << 16);

const fn sprite_row(row: &[u8; CLAWD_COLS as usize]) -> u32 {
    let mut mask = 0;
    let mut col = 0;

    while col < row.len() {
        if row[col] == b'#' {
            mask |= 1 << col;
        }

        col += 1;
    }

    mask
}

/// The Claude Code mascot. One sprite cell is a terminal quadrant, so cells are twice as tall as wide.
struct Clawd {
    top_left: Point,
    cell: Size,
    blink: bool,
    arms_up: bool,
}

impl Clawd {
    fn rows(&self) -> [u32; 5] {
        let mut rows = CLAWD;

        if self.blink {
            rows[1] |= EYES;
        }

        if self.arms_up {
            rows[1] |= ARMS;
            rows[2] &= !ARMS;
        }

        rows
    }
}

impl Drawable for Clawd {
    type Color = BinaryColor;
    type Output = ();

    fn draw<D>(&self, target: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = BinaryColor>,
    {
        let Size { width, height } = self.cell;

        // One fill per horizontal run keeps a frame well inside the display channel capacity.
        self.rows()
            .into_iter()
            .zip(0..)
            .flat_map(|(mask, row)| {
                (0..CLAWD_COLS)
                    .filter(move |&col| mask & (1 << col) != 0 && (col == 0 || mask & (1 << (col - 1)) == 0))
                    .map(move |start| (row, start, (mask >> start).trailing_ones()))
            })
            .try_for_each(|(row, start, length)| {
                let corner = self.top_left + Point::new((start * width).cast_signed(), (row * height).cast_signed());

                target.fill_solid(&Rectangle::new(corner, Size::new(length * width, height)), BinaryColor::On)
            })
    }
}

/// The asterisk spinner Claude Code shows while it works, grown and shrunk per frame.
fn draw_spinner(display: &mut DisplayProxy, center: Point, tick: u32) {
    const RADII: [i32; 6] = [0, 1, 2, 3, 3, 2];

    let radius = RADII[tick as usize % RADII.len()];
    let diagonal = radius * 2 / 3;
    let stroke = PrimitiveStyle::with_stroke(BinaryColor::On, 1);

    let arms = [
        (Point::new(-radius, 0), Point::new(radius, 0)),
        (Point::new(0, -radius), Point::new(0, radius)),
        (Point::new(-diagonal, -diagonal), Point::new(diagonal, diagonal)),
        (Point::new(-diagonal, diagonal), Point::new(diagonal, -diagonal)),
    ];

    for (start, end) in arms {
        Line::new(center + start, center + end).into_styled(stroke).draw(display).unwrap();
    }
}

const CTRL_B: Keystroke = Keystroke::with(CTRL, 0x05);
const CTRL_O: Keystroke = Keystroke::with(CTRL, 0x12);
const CTRL_T: Keystroke = Keystroke::with(CTRL, 0x17);
const SHIFT_TAB: Keystroke = Keystroke::with(SHIFT, 0x2B);

enum Macro {
    Keys(&'static [Keystroke]),
    Text(&'static str),
}

struct Binding {
    label: &'static str,
    action: Macro,
}

impl Binding {
    fn for_key(key: Key) -> Option<Self> {
        let (label, action) = match key {
            Key::F1 => ("/clear", Macro::Text("/clear")),
            Key::F2 => ("/compact", Macro::Text("/compact")),
            Key::F3 => ("/context", Macro::Text("/context")),
            Key::F4 => ("/model", Macro::Text("/model")),
            Key::Div => ("/ command", Macro::Text("/")),
            Key::Mul => ("transcript", Macro::Keys(&[CTRL_O])),
            Key::Sub => ("Esc: stop", Macro::Keys(&[Keystroke::ESCAPE])),
            Key::Add => ("cycle mode", Macro::Keys(&[SHIFT_TAB])),
            Key::D0 => ("rewind", Macro::Keys(&[Keystroke::ESCAPE, Keystroke::ESCAPE])),
            Key::D1 => ("1: yes", Macro::Text("1")),
            Key::D2 => ("2: always", Macro::Text("2")),
            Key::D3 => ("3: no", Macro::Text("3")),
            Key::D4 => ("left", Macro::Keys(&[Keystroke::LEFT])),
            Key::D5 => ("down", Macro::Keys(&[Keystroke::DOWN])),
            Key::D6 => ("right", Macro::Keys(&[Keystroke::RIGHT])),
            Key::D7 => ("background", Macro::Keys(&[CTRL_B])),
            Key::D8 => ("up", Macro::Keys(&[Keystroke::UP])),
            Key::D9 => ("task list", Macro::Keys(&[CTRL_T])),
            Key::Dot => ("tab", Macro::Keys(&[Keystroke::TAB])),
            Key::Enter => ("enter", Macro::Keys(&[Keystroke::ENTER])),
            Key::Lock | Key::NC => return None,
        };

        Some(Self { label, action })
    }

    async fn send(&self) {
        match self.action {
            Macro::Keys(strokes) => {
                for stroke in strokes {
                    stroke.send().await;
                }
            }
            Macro::Text(text) => {
                for stroke in text.chars().filter_map(Keystroke::from_char) {
                    stroke.send().await;
                }
            }
        }
    }
}

fn format_elapsed(elapsed: Duration) -> String<8> {
    let seconds = elapsed.as_secs();
    let mut text = String::new();

    let _ = match seconds {
        0..60 => write!(text, "{seconds}s"),
        60..3600 => write!(text, "{}m{:02}s", seconds / 60, seconds % 60),
        _ => write!(text, "{}h{:02}m", seconds / 3600, seconds / 60 % 60),
    };

    text
}

pub struct Claude {
    tick: u32,
    feedback: Option<(&'static str, Instant)>,
}

impl Claude {
    pub fn new() -> Self {
        Self {
            tick: 0,
            feedback: None,
        }
    }

    async fn intro(&self, display: &mut DisplayProxy<'_>) {
        let style = text_style(BinaryColor::On);

        let drop = [-30, -20, -10, 0, 3, 1, 1].map(|y| (y, 0, false, false));
        let typing = (1..=TITLE.len()).map(|length| (1, length, false, false));
        let wave = [false, true, false, true, false, false, true, false].map(|wave| (1, TITLE.len(), wave, wave));

        for (y, title_length, blink, arms_up) in drop.into_iter().chain(typing).chain(wave) {
            if !MODE_RUNNING.load(Ordering::Relaxed) {
                return;
            }

            display.idle().await;
            display.clear(BinaryColor::Off).unwrap();

            Clawd {
                top_left: Point::new(2, y),
                cell: Size::new(3, 6),
                blink,
                arms_up,
            }
            .draw(display)
            .unwrap();

            Text::with_baseline(&TITLE[..title_length], Point::new(61, 11), style, Baseline::Top)
                .draw(display)
                .unwrap();

            display.flush().unwrap();

            Timer::after_millis(70).await;
        }
    }

    fn draw(&self, display: &mut DisplayProxy, sessions: &Sessions) {
        let now = Instant::now();
        let flash = self.tick % 4 < 2;
        let normal = text_style(BinaryColor::On);
        let focus = sessions.focus();

        display.clear(BinaryColor::Off).unwrap();

        let (bob, arms_up) = match focus.map(|session| session.activity) {
            Some(Activity::Busy) => (i32::from(self.tick / 2 % 2 == 1), false),
            Some(Activity::Waiting) => (-i32::from(flash), flash),
            _ => (0, false),
        };

        Clawd {
            top_left: Point::new(0, 1 + bob),
            cell: Size::new(2, 4),
            blink: self.tick.is_multiple_of(24),
            arms_up,
        }
        .draw(display)
        .unwrap();

        let (label, label_position, label_style) = match focus {
            None => ("No sessions", Point::new(40, 0), normal),
            Some(session) => match session.activity {
                Activity::Ready => ("Ready", Point::new(40, 0), normal),
                Activity::Done => ("Done", Point::new(40, 0), normal),
                Activity::Busy => {
                    draw_spinner(display, Point::new(43, 4), self.tick);

                    let verb = VERBS[usize::from(session.turn_started.as_ticks().to_le_bytes()[0]) % VERBS.len()];

                    (verb, Point::new(50, 0), normal)
                }
                Activity::Waiting if flash => {
                    display.fill_solid(&Rectangle::new(Point::new(38, 0), Size::new(82, 10)), BinaryColor::On).unwrap();

                    ("Needs you!", Point::new(40, 0), text_style(BinaryColor::Off))
                }
                Activity::Waiting => ("Needs you!", Point::new(40, 0), normal),
            },
        };

        let elapsed = match focus {
            Some(session) if matches!(session.activity, Activity::Busy | Activity::Waiting) => {
                format_elapsed(now - session.turn_started)
            }
            Some(session) if session.activity == Activity::Done => format_elapsed(session.since - session.turn_started),
            _ => String::new(),
        };

        let detail = match (self.feedback, focus) {
            (Some((feedback, at)), _) if now - at < FEEDBACK_TIME => feedback,
            (_, Some(session)) => session.detail.as_str(),
            (_, None) => "start claude",
        };

        // Session grid at the right edge: filled while working, outlined while idle, blinking while it waits.
        sessions.iter().zip(0..).for_each(|(session, index)| {
            let filled = match session.activity {
                Activity::Busy => true,
                Activity::Waiting => flash,
                Activity::Ready | Activity::Done => false,
            };
            let style = if filled {
                PrimitiveStyle::with_fill(BinaryColor::On)
            } else {
                PrimitiveStyle::with_stroke(BinaryColor::On, 1)
            };

            Rectangle::new(Point::new(DisplayProxy::WIDTH - 7 + index / 8 * 4, index % 8 * 4), Size::new(3, 3))
                .into_styled(style)
                .draw(display)
                .unwrap();
        });

        let lines = [
            (label, label_position, label_style),
            (detail, Point::new(40, 11), normal),
            (focus.map_or("", |session| session.project.as_str()), Point::new(40, 22), normal),
            (elapsed.as_str(), Point::new(3, 22), normal),
        ];

        for (content, position, style) in lines {
            Text::with_baseline(&content[..content.len().min(TEXT_COLUMNS)], position, style, Baseline::Top)
                .draw(display)
                .unwrap();
        }

        display.flush().unwrap();
    }
}

fn text_style(color: BinaryColor) -> MonoTextStyle<'static, BinaryColor> {
    MonoTextStyleBuilder::new().font(&FONT_6X10).text_color(color).build()
}

impl Mode for Claude {
    async fn task(&mut self) {
        let mut display = DisplayProxy::new();

        HID_CHANNEL.sender().send(HidEvent::Reset).await;

        self.intro(&mut display).await;

        let mut keypad = KEYPAD_CHANNEL.subscriber().unwrap();

        while MODE_RUNNING.load(Ordering::Relaxed) {
            display.idle().await;
            self.draw(&mut display, &claude_sessions());

            match select3(keypad.next_message(), CLAUDE_CHANGED.wait(), Timer::after(FRAME)).await {
                Either3::First(WaitResult::Message(event)) if event.pressed && !key_pressed(Key::Lock) => {
                    if let Some(binding) = Binding::for_key(event.key) {
                        self.feedback = Some((binding.label, Instant::now()));
                        binding.send().await;
                    }
                }
                Either3::Third(()) => self.tick = self.tick.wrapping_add(1),
                _ => {}
            }
        }
    }
}
