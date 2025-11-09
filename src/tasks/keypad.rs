use crate::utils::debounce::Debounce;
use embassy_executor::Spawner;
use embassy_rp::gpio::{Level, Pull, Output, Input, Pin};
use embassy_rp::Peri;
use embassy_time::{Duration, Instant};
use static_cell::StaticCell;
use crate::tasks::display;

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
}

const ROWS: usize = 6;
const COLS: usize = 4;

static ROWS_CELL: StaticCell<[Output<'static>; ROWS]> = StaticCell::new();
static COLS_CELL: StaticCell<[Input<'static>; COLS]> = StaticCell::new();

const KEYMAP: [[Key; COLS]; ROWS] = [
    [Key::F1, Key::F2, Key::F3, Key::F4],
    [Key::LOCK, Key::DIV, Key::MUL, Key::SUB],
    [Key::D7, Key::D8, Key::D9, Key::ADD],
    [Key::D4, Key::D5, Key::D6, Key::ADD],
    [Key::D1, Key::D2, Key::D3, Key::ENTER],
    [Key::D0, Key::D0, Key::DOT, Key::ENTER],
];

pub async fn init(spawner: &Spawner, row_pins: [Peri<'static, impl Pin>; ROWS], col_pins: [Peri<'static, impl Pin>; COLS]) {
    let rows = ROWS_CELL.init(row_pins.map(|pin| Output::new(pin, Level::High)));
    let cols = COLS_CELL.init(col_pins.map(|pin| Input::new(pin, Pull::Up)));

    spawner.spawn(keyboard_task(rows, cols).unwrap());
}

#[embassy_executor::task]
pub async fn keyboard_task(
    rows: &'static mut [Output<'static>; ROWS],
    cols: &'static [Input<'static>; COLS],
) {

}