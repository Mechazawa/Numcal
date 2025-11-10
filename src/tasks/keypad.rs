use crate::utils::debounce::Debounce;
use embassy_executor::Spawner;
use embassy_rp::gpio::{Level, Pull, Output, Input, AnyPin};
use embassy_rp::Peri;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::pubsub::PubSubChannel;
use embassy_time::{Duration, Timer};
use log::info;
use portable_atomic::{AtomicBool, Ordering};
use static_cell::StaticCell;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Key {
    F1,
    F2,
    F3,
    F4,
    LOCK,
    DIV,
    MUL,
    SUB,
    ADD,
    ENTER,
    DOT,
    D0,
    D1,
    D2,
    D3,
    D4,
    D5,
    D6,
    D7,
    D8,
    D9,
    NC,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KeyEvent {
    pub key: Key,
    pub pressed: bool,
}

const ROWS: usize = 6;
const COLS: usize = 4;
const KEY_COUNT: usize = ROWS * COLS;
const DEBOUNCE_DELAY: Duration = Duration::from_millis(10);
const LOOP_DELAY: Duration = Duration::from_millis(2);

static ROWS_CELL: StaticCell<[Output<'static>; ROWS]> = StaticCell::new();
static COLS_CELL: StaticCell<[Input<'static>; COLS]> = StaticCell::new();
static DEBOUNCE_CELL: StaticCell<[[Debounce<bool>; COLS]; ROWS]> = StaticCell::new();

pub static KEYPAD_CHANNEL: PubSubChannel<CriticalSectionRawMutex, KeyEvent, 32, 10, 1> = PubSubChannel::new();

// todo not great but does the trick
static STATE: [[AtomicBool; COLS]; ROWS] = [
    [AtomicBool::new(false), AtomicBool::new(false), AtomicBool::new(false), AtomicBool::new(false)],
    [AtomicBool::new(false), AtomicBool::new(false), AtomicBool::new(false), AtomicBool::new(false)],
    [AtomicBool::new(false), AtomicBool::new(false), AtomicBool::new(false), AtomicBool::new(false)],
    [AtomicBool::new(false), AtomicBool::new(false), AtomicBool::new(false), AtomicBool::new(false)],
    [AtomicBool::new(false), AtomicBool::new(false), AtomicBool::new(false), AtomicBool::new(false)],
    [AtomicBool::new(false), AtomicBool::new(false), AtomicBool::new(false), AtomicBool::new(false)],
];

const KEYMAP: [[Key; COLS]; ROWS] = [
    [Key::F1, Key::F2, Key::F3, Key::F4],
    [Key::LOCK, Key::DIV, Key::MUL, Key::SUB],
    [Key::D7, Key::D8, Key::D9, Key::NC],
    [Key::D4, Key::D5, Key::D6, Key::ADD],
    [Key::D1, Key::D2, Key::D3, Key::NC],
    [Key::NC, Key::D0, Key::DOT, Key::ENTER],
];

pub async fn init(
    spawner: &Spawner,
    row_pins: [Peri<'static, AnyPin>; ROWS],
    col_pins: [Peri<'static, AnyPin>; COLS],
) {
    let rows = ROWS_CELL.init(row_pins.map(|pin| Output::new(pin, Level::High)));
    let cols = COLS_CELL.init(col_pins.map(|pin| Input::new(pin, Pull::Up)));
    let debouncer = DEBOUNCE_CELL.init([[false; COLS]; ROWS].map(|row| row.map(|value| Debounce::new(value, DEBOUNCE_DELAY))));

    spawner.spawn(keyboard_task(rows, cols, debouncer).unwrap());
}

#[embassy_executor::task]
pub async fn keyboard_task(
    rows: &'static mut [Output<'static>; ROWS],
    cols: &'static [Input<'static>; COLS],
    debouncer: &'static mut [[Debounce<bool>; COLS]; ROWS],
) {
    let publisher = KEYPAD_CHANNEL.publisher().unwrap();

    loop {
        let mut events = heapless::Vec::<KeyEvent, KEY_COUNT>::new();

        for (row_idx, row_pin) in rows.iter_mut().enumerate() {
            row_pin.set_low();

            for (col_idx, col_pin) in cols.iter().enumerate() {
                let pressed = col_pin.is_low();
                let changed = debouncer[row_idx][col_idx].measure(pressed);

                if changed {
                    STATE[row_idx][col_idx].store(pressed, Ordering::Relaxed);

                    let key = KEYMAP[row_idx][col_idx];

                    events.push(KeyEvent {key, pressed}).unwrap();

                    if pressed {
                        info!("DOWN [{row_idx}][{col_idx}] {:?}", key);
                    } else {
                        info!("UP   [{row_idx}][{col_idx}] {:?}", key);
                    }
                }
            }

            row_pin.set_high();
        }

        for event in events {
            publisher.publish_immediate(event);
        }

        Timer::after(LOOP_DELAY).await;
    }
}

pub fn key_pressed(key: Key) -> bool {
    // Todo improve the lookup
    for col in 0..COLS {
        for row in 0..ROWS {
            if KEYMAP[row][col] == key {
                return STATE[row][col].load(Ordering::Relaxed);
            }
        }
    }

    false
}