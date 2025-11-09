use crate::utils::debounce::Debounce;
use embassy_executor::Spawner;
use embassy_rp::gpio::{Level, Pull, Output, Input, AnyPin, Pin};
use embassy_rp::Peri;
use embassy_time::{Duration, Instant, Timer};
use log::info;
use static_cell::StaticCell;
use crate::tasks::display;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Key {
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

const ROWS: usize = 6;
const COLS: usize = 4;
const DEBOUNCE_DELAY: Duration = Duration::from_millis(10);
const LOOP_DELAY: Duration = Duration::from_millis(2);

static ROWS_CELL: StaticCell<[Output<'static>; ROWS]> = StaticCell::new();
static COLS_CELL: StaticCell<[Input<'static>; COLS]> = StaticCell::new();
static DEBOUNCE_CELL: StaticCell<[[Debounce<bool>; COLS]; ROWS]> = StaticCell::new();
static mut STATE: [[bool; COLS]; ROWS] = [[false; COLS]; ROWS];

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
    loop {
        for (row_idx, row_pin) in rows.iter_mut().enumerate() {
            row_pin.set_low();

            for (col_idx, col_pin) in cols.iter().enumerate() {
                let pressed = col_pin.is_low();
                let changed = debouncer[row_idx][col_idx].measure(pressed);

                if changed {
                    unsafe {
                        STATE[row_idx][col_idx] = pressed;
                    }

                    let key = KEYMAP[row_idx][col_idx];

                    if pressed {
                        info!("DOWN [{row_idx}][{col_idx}] {:?}", key);
                    } else {
                        info!("UP   [{row_idx}][{col_idx}] {:?}", key);
                    }
                }
            }

            row_pin.set_high();
        }

        Timer::after(LOOP_DELAY).await;
    }
}

pub fn pressed(key: Key) -> bool {
    // Todo improve the lookup
    for col in 0..COLS {
        for row in 0..ROWS {
            if KEYMAP[row][col] == key {
                return unsafe { STATE[row][col] };
            }
        }
    }

    false
}