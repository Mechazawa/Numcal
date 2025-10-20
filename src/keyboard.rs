use embassy_rp::gpio::{Input, Output};
use embassy_sync::channel::Sender;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_time::Timer;

use crate::modes::{calculator::Calculator, Mode};
use crate::{KeyEvent, COLS, DEBOUNCE_MS, KEYMAP, ROWS};

/// Represents a single key in the matrix with its state
#[derive(Clone, Copy)]
struct Key {
    row: usize,
    col: usize,
    is_pressed: bool,
    debounce_timer: u64,
}

impl Key {
    fn new(row: usize, col: usize) -> Self {
        Self {
            row,
            col,
            is_pressed: false,
            debounce_timer: 0,
        }
    }

    /// Returns the USB HID keycode for this key
    fn keycode(&self) -> u8 {
        KEYMAP[self.row][self.col]
    }

    /// Maps this key to a calculator character if applicable
    fn to_calc_char(&self) -> Option<char> {
        match (self.row, self.col) {
            // Row 1: Clear (Numlock), /, *, -
            (1, 0) => Some('C'), // Numlock = Clear
            (1, 1) => Some('/'),
            (1, 2) => Some('*'),
            (1, 3) => Some('-'),
            // Row 2: 7, 8, 9
            (2, 0) => Some('7'),
            (2, 1) => Some('8'),
            (2, 2) => Some('9'),
            // Row 3: 4, 5, 6, +
            (3, 0) => Some('4'),
            (3, 1) => Some('5'),
            (3, 2) => Some('6'),
            (3, 3) => Some('+'),
            // Row 4: 1, 2, 3
            (4, 0) => Some('1'),
            (4, 1) => Some('2'),
            (4, 2) => Some('3'),
            // Row 5: 0, ., Enter (=)
            (5, 1) => Some('0'),
            (5, 2) => Some('.'),
            (5, 3) => Some('='),
            _ => None,
        }
    }

    /// Check if this is the Numlock key (R1C0)
    fn is_numlock(&self) -> bool {
        self.row == 1 && self.col == 0
    }

    /// Check if this is a mode switch key (row 0)
    fn is_mode_key(&self) -> bool {
        self.row == 0
    }
}

/// Manages the keyboard matrix scanning and debouncing
struct KeyMatrix {
    keys: [[Key; COLS]; ROWS],
}

impl KeyMatrix {
    fn new() -> Self {
        let mut keys = [[Key::new(0, 0); COLS]; ROWS];
        for row in 0..ROWS {
            for col in 0..COLS {
                keys[row][col] = Key::new(row, col);
            }
        }
        Self { keys }
    }

    /// Scan the matrix and update key states. Returns keys that changed state.
    async fn scan(
        &mut self,
        rows: &mut [Output<'static>; ROWS],
        cols: &[Input<'static>; COLS],
    ) -> KeyEvents {
        let mut events = KeyEvents::new();

        for (row_idx, row_pin) in rows.iter_mut().enumerate() {
            // Drive this row LOW
            row_pin.set_low();
            Timer::after_micros(10).await;

            // Read all columns
            for (col_idx, col_pin) in cols.iter().enumerate() {
                let key = &mut self.keys[row_idx][col_idx];
                let is_low = col_pin.is_low();

                // Update debounce logic
                if is_low != key.is_pressed {
                    // State differs from stable state
                    if key.debounce_timer == 0 {
                        // Start debounce timer
                        key.debounce_timer = DEBOUNCE_MS;
                    } else {
                        // Decrement timer
                        key.debounce_timer -= 1;

                        // Timer expired, update stable state
                        if key.debounce_timer == 0 {
                            let was_pressed = key.is_pressed;
                            key.is_pressed = is_low;

                            if is_low && !was_pressed {
                                defmt::info!(
                                    "Key pressed: R{}C{} (keycode=0x{:02x})",
                                    row_idx,
                                    col_idx,
                                    key.keycode()
                                );
                                let _ = events.presses.push(*key);
                            } else if !is_low && was_pressed {
                                defmt::info!(
                                    "Key released: R{}C{} (keycode=0x{:02x})",
                                    row_idx,
                                    col_idx,
                                    key.keycode()
                                );
                                let _ = events.releases.push(*key);
                            }
                        }
                    }
                } else {
                    // State matches stable state, reset timer
                    key.debounce_timer = 0;
                }
            }

            // Set row back to HIGH
            row_pin.set_high();
        }

        events
    }

    /// Get all currently pressed keys
    fn get_pressed_keys(&self) -> heapless::Vec<(usize, usize), 24> {
        let mut pressed = heapless::Vec::new();
        for row in 0..ROWS {
            for col in 0..COLS {
                if self.keys[row][col].is_pressed {
                    let _ = pressed.push((row, col));
                }
            }
        }
        pressed
    }

    /// Check if all top row keys are pressed (bootsel reboot trigger)
    fn all_top_row_pressed(&self) -> bool {
        (0..4).all(|col| self.keys[0][col].is_pressed)
    }
}

/// Collection of key events from a scan
struct KeyEvents {
    presses: heapless::Vec<Key, 24>,
    releases: heapless::Vec<Key, 24>,
}

impl KeyEvents {
    fn new() -> Self {
        Self {
            presses: heapless::Vec::new(),
            releases: heapless::Vec::new(),
        }
    }
}

/// Manages high-level keyboard state (modes, calculator, special keys)
struct KeyboardState {
    current_mode: Mode,
    calculator: Calculator,
    numlock_held: bool,
}

impl KeyboardState {
    fn new() -> Self {
        Self {
            current_mode: Mode::default(),
            calculator: Calculator::new(),
            numlock_held: false,
        }
    }

    /// Handle a key press event
    async fn handle_press(
        &mut self,
        key: Key,
        usb_sender: &Sender<'_, ThreadModeRawMutex, KeyEvent, 8>,
    ) {
        // Check for mode switching: Hold Numlock + Row 0 keys
        if key.is_mode_key() && self.numlock_held {
            self.switch_mode(key.col);
            return;
        }

        // Handle Numlock key
        if key.is_numlock() {
            self.numlock_held = true;
            // In calculator mode, Numlock also acts as 'C' (clear)
            if self.current_mode == Mode::Calculator {
                if let Some(ch) = key.to_calc_char() {
                    self.calculator.handle_key(ch);
                }
            }
            return;
        }

        // Handle regular key press based on current mode
        match self.current_mode {
            Mode::Numpad => {
                usb_sender
                    .send(KeyEvent {
                        row: key.row,
                        col: key.col,
                        pressed: true,
                    })
                    .await;
            }
            Mode::Calculator => {
                if let Some(ch) = key.to_calc_char() {
                    self.calculator.handle_key(ch);
                }
            }
            _ => {
                // Other modes - do nothing for now
            }
        }
    }

    /// Handle a key release event
    async fn handle_release(
        &mut self,
        key: Key,
        usb_sender: &Sender<'_, ThreadModeRawMutex, KeyEvent, 8>,
    ) {
        // Check if Numlock released
        if key.is_numlock() {
            self.numlock_held = false;
            return;
        }

        // In numpad mode, send key release to USB
        if self.current_mode == Mode::Numpad {
            usb_sender
                .send(KeyEvent {
                    row: key.row,
                    col: key.col,
                    pressed: false,
                })
                .await;
        }
    }

    /// Switch to a new mode based on column index
    fn switch_mode(&mut self, col: usize) {
        let new_mode = match col {
            0 => Mode::Numpad,
            1 => Mode::Calculator,
            2 => Mode::M2,
            3 => Mode::M3,
            _ => return,
        };

        if new_mode != self.current_mode {
            defmt::info!("Mode switched to: {:?}", new_mode);
            self.current_mode = new_mode;
        }
    }

    /// Format display text for the current mode
    fn format_display(&self, pressed_keys: &heapless::Vec<(usize, usize), 24>) -> heapless::String<64> {
        match self.current_mode {
            Mode::Numpad => crate::modes::numpad::format_display(pressed_keys),
            Mode::Calculator => self.calculator.format_display(),
            Mode::M2 | Mode::M3 => {
                let mut text = heapless::String::<64>::new();
                use core::fmt::Write;
                write!(&mut text, "{} Reserved", self.current_mode.name()).unwrap();
                text
            }
        }
    }
}

#[embassy_executor::task]
pub async fn keyboard_task(
    rows: &'static mut [Output<'static>; ROWS],
    cols: &'static [Input<'static>; COLS],
) {
    defmt::info!("Keyboard task started");

    // Initialize state
    let mut matrix = KeyMatrix::new();
    let mut state = KeyboardState::new();

    let display_sender = crate::DISPLAY_CHANNEL.sender();
    let usb_sender = crate::USB_CHANNEL.sender();

    defmt::info!("Keyboard matrix scanner initialized");

    // Debug: Check initial column states
    Timer::after_millis(100).await;
    for (idx, col) in cols.iter().enumerate() {
        defmt::info!("Initial col {} state: high={}", idx, col.is_high());
    }

    loop {
        // Scan the matrix and get key events
        let events = matrix.scan(rows, cols).await;

        // Handle key presses
        for key in events.presses.iter() {
            state.handle_press(*key, &usb_sender).await;
        }

        // Handle key releases
        for key in events.releases.iter() {
            state.handle_release(*key, &usb_sender).await;
        }

        // Check for bootsel reboot: all 4 top row keys pressed
        if matrix.all_top_row_pressed() {
            defmt::info!("All top row buttons pressed - rebooting to bootsel mode!");

            let mut bootsel_text = heapless::String::<64>::new();
            use core::fmt::Write;
            write!(&mut bootsel_text, "BOOTSEL REBOOT").unwrap();
            display_sender.send(bootsel_text).await;

            Timer::after_millis(100).await;
            embassy_rp::rom_data::reset_to_usb_boot(0, 0);
        }

        // Update display
        let pressed_keys = matrix.get_pressed_keys();
        let display_text = state.format_display(&pressed_keys);
        display_sender.send(display_text).await;

        // Scan rate: 1ms between scans
        Timer::after_millis(1).await;
    }
}
