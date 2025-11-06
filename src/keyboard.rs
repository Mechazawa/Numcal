use embassy_rp::gpio::{Input, Output};
use embassy_sync::channel::Sender;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_time::Timer;
use log::info;
use crate::modes::{calculator::Calculator, Mode};
use crate::{KeyEvent, COLS, DEBOUNCE_MS, KEYMAP, ROWS};

/// Represents a single key in the matrix with its state
#[derive(Clone, Copy)]
struct Key {
    is_pressed: bool,
    debounce_timer: u8,
}

impl Key {
    fn new() -> Self {
        Self {
            is_pressed: false,
            debounce_timer: 0,
        }
    }

    /// Update key state based on current input and debounce timer.
    /// Returns Some(new_pressed_state) if the stable state changed after debounce.
    fn update(&mut self, is_low: bool) -> Option<bool> {
        if is_low != self.is_pressed {
            // State differs from stable state
            if self.debounce_timer == 0 {
                // Start debounce timer
                self.debounce_timer = DEBOUNCE_MS;
                None
            } else {
                // Decrement timer
                self.debounce_timer -= 1;

                // Timer expired, update stable state
                if self.debounce_timer == 0 {
                    self.is_pressed = is_low;
                    Some(is_low)
                } else {
                    None
                }
            }
        } else {
            // State matches stable state, reset timer
            self.debounce_timer = 0;
            None
        }
    }

    /// Returns the USB HID keycode for this key at the given position
    fn keycode(row: usize, col: usize) -> u8 {
        KEYMAP[row][col]
    }

    /// Maps this key position to a calculator character if applicable
    fn to_calc_char(row: usize, col: usize) -> Option<char> {
        match (row, col) {
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

    /// Check if this position is the Numlock key (R1C0)
    fn is_numlock(row: usize, col: usize) -> bool {
        row == 1 && col == 0
    }

    /// Check if this position is a mode switch key (row 0)
    fn is_mode_key(row: usize) -> bool {
        row == 0
    }
}

/// Manages the keyboard matrix scanning and debouncing
struct KeyMatrix {
    keys: [[Key; COLS]; ROWS],
}

impl KeyMatrix {
    fn new() -> Self {
        Self {
            keys: [[Key::new(); COLS]; ROWS],
        }
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

                // Update key state with debouncing
                if let Some(new_state) = key.update(is_low) {
                    // State changed after debouncing
                    if new_state {
                        info!(
                            "Key pressed: R{}C{} (keycode=0x{:02x})",
                            row_idx,
                            col_idx,
                            Key::keycode(row_idx, col_idx)
                        );
                        let _ = events.presses.push((row_idx, col_idx));
                    } else {
                        info!(
                            "Key released: R{}C{} (keycode=0x{:02x})",
                            row_idx,
                            col_idx,
                            Key::keycode(row_idx, col_idx)
                        );
                        let _ = events.releases.push((row_idx, col_idx));
                    }
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
    presses: heapless::Vec<(usize, usize), 24>,
    releases: heapless::Vec<(usize, usize), 24>,
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
        row: usize,
        col: usize,
        usb_sender: &Sender<'_, ThreadModeRawMutex, KeyEvent, 8>,
    ) {
        // Check for mode switching: Hold Numlock + Row 0 keys
        if Key::is_mode_key(row) && self.numlock_held {
            self.switch_mode(col);
            return;
        }

        // Handle Numlock key
        if Key::is_numlock(row, col) {
            self.numlock_held = true;
            // In calculator mode, Numlock also acts as 'C' (clear)
            if self.current_mode == Mode::Calculator {
                if let Some(ch) = Key::to_calc_char(row, col) {
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
                        row,
                        col,
                        pressed: true,
                    })
                    .await;
            }
            Mode::Calculator => {
                if let Some(ch) = Key::to_calc_char(row, col) {
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
        row: usize,
        col: usize,
        usb_sender: &Sender<'_, ThreadModeRawMutex, KeyEvent, 8>,
    ) {
        // Check if Numlock released
        if Key::is_numlock(row, col) {
            self.numlock_held = false;
            // return;
        }

        // In numpad mode, send key release to USB
        if self.current_mode == Mode::Numpad {
            usb_sender
                .send(KeyEvent {
                    row,
                    col,
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
            info!("Mode switched to: {:?}", new_mode);
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
    info!("Keyboard task started");

    // Initialize state
    let mut matrix = KeyMatrix::new();
    let mut state = KeyboardState::new();

    let display_sender = crate::DISPLAY_CHANNEL.sender();
    let usb_sender = crate::USB_CHANNEL.sender();

    info!("Keyboard matrix scanner initialized");

    // Debug: Check initial column states
    Timer::after_millis(100).await;
    for (idx, col) in cols.iter().enumerate() {
        info!("Initial col {} state: high={}", idx, col.is_high());
    }

    loop {
        // Scan the matrix and get key events
        let events = matrix.scan(rows, cols).await;

        // Handle key presses
        for &(row, col) in events.presses.iter() {
            state.handle_press(row, col, &usb_sender).await;
        }

        // Handle key releases
        for &(row, col) in events.releases.iter() {
            state.handle_release(row, col, &usb_sender).await;
        }

        // Check for bootsel reboot: all 4 top row keys pressed
        if matrix.all_top_row_pressed() {
            info!("All top row buttons pressed - rebooting to bootsel mode!");

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
