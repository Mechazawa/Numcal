use embassy_rp::gpio::{Input, Output};
use embassy_time::Timer;

use crate::modes::Mode;
use crate::{KeyEvent, COLS, DEBOUNCE_MS, DISPLAY_CHANNEL, KEYMAP, ROWS, USB_CHANNEL};

#[embassy_executor::task]
pub async fn keyboard_task(
    rows: &'static mut [Output<'static>; ROWS],
    cols: &'static [Input<'static>; COLS],
) {
    defmt::info!("Keyboard task started");

    // Track key states for debouncing: [row][col] -> (is_pressed, debounce_timer)
    let mut key_states = [[false; COLS]; ROWS];
    let mut debounce_timers = [[0u64; COLS]; ROWS];

    // Current mode
    let mut current_mode = Mode::default();

    // Track if Numlock is held (for mode switching)
    let mut numlock_held = false;

    let display_sender = DISPLAY_CHANNEL.sender();
    let usb_sender = USB_CHANNEL.sender();

    defmt::info!("Keyboard matrix scanner initialized");

    // Debug: Check initial column states
    Timer::after_millis(100).await;
    for (idx, col) in cols.iter().enumerate() {
        defmt::info!("Initial col {} state: high={}", idx, col.is_high());
    }

    loop {
        let mut pressed_keys = heapless::Vec::<(usize, usize), 24>::new(); // Max 24 keys (6x4)

        // Scan the matrix
        for (row_idx, row_pin) in rows.iter_mut().enumerate() {
            // Drive this row LOW
            row_pin.set_low();

            // Small delay to let the signal settle
            Timer::after_micros(10).await;

            // Read all columns
            for (col_idx, col_pin) in cols.iter().enumerate() {
                let is_low = col_pin.is_low();

                // Debug logging when we see a LOW column (only on first detection)
                if is_low
                    && debounce_timers[row_idx][col_idx] == 0
                    && !key_states[row_idx][col_idx]
                {
                    defmt::trace!("Detected LOW at R{}C{}", row_idx, col_idx);
                }

                // Update debounce logic
                if is_low != key_states[row_idx][col_idx] {
                    // State differs from stable state
                    if debounce_timers[row_idx][col_idx] == 0 {
                        // Start debounce timer
                        debounce_timers[row_idx][col_idx] = DEBOUNCE_MS;
                    } else {
                        // Decrement timer
                        debounce_timers[row_idx][col_idx] -= 1;

                        // Timer expired, update stable state
                        if debounce_timers[row_idx][col_idx] == 0 {
                            let was_pressed = key_states[row_idx][col_idx];
                            key_states[row_idx][col_idx] = is_low;

                            if is_low && !was_pressed {
                                defmt::info!(
                                    "Key pressed: R{}C{} (keycode=0x{:02x})",
                                    row_idx,
                                    col_idx,
                                    KEYMAP[row_idx][col_idx]
                                );

                                // Check for mode switching: Hold Numlock (R1C0) + Row 0 keys
                                if row_idx == 0 && numlock_held {
                                    // Mode switch!
                                    let new_mode = match col_idx {
                                        0 => Mode::Numpad,
                                        1 => Mode::Calculator,
                                        2 => Mode::M2,
                                        3 => Mode::M3,
                                        _ => current_mode,
                                    };
                                    if new_mode != current_mode {
                                        defmt::info!("Mode switched to: {:?}", new_mode);
                                        current_mode = new_mode;
                                    }
                                } else if row_idx == 1 && col_idx == 0 {
                                    // Numlock pressed
                                    numlock_held = true;
                                } else {
                                    // Regular key press - send to USB based on mode
                                    if current_mode == Mode::Numpad {
                                        usb_sender
                                            .send(KeyEvent {
                                                row: row_idx,
                                                col: col_idx,
                                                pressed: true,
                                            })
                                            .await;
                                    }
                                }
                            } else if !is_low && was_pressed {
                                defmt::info!(
                                    "Key released: R{}C{} (keycode=0x{:02x})",
                                    row_idx,
                                    col_idx,
                                    KEYMAP[row_idx][col_idx]
                                );

                                // Check if Numlock released
                                if row_idx == 1 && col_idx == 0 {
                                    numlock_held = false;
                                } else if current_mode == Mode::Numpad {
                                    // Regular key release - send to USB
                                    usb_sender
                                        .send(KeyEvent {
                                            row: row_idx,
                                            col: col_idx,
                                            pressed: false,
                                        })
                                        .await;
                                }
                            }
                        }
                    }
                } else {
                    // State matches stable state, reset timer
                    debounce_timers[row_idx][col_idx] = 0;
                }

                // Collect currently pressed keys
                if key_states[row_idx][col_idx] {
                    let _ = pressed_keys.push((row_idx, col_idx));
                }
            }

            // Set row back to HIGH
            row_pin.set_high();
        }

        // Check for bootsel reboot: all 4 top row buttons (R0C0, R0C1, R0C2, R0C3) pressed
        let top_row_pressed = (0..4).all(|col| key_states[0][col]);
        if top_row_pressed {
            defmt::info!("All top row buttons pressed - rebooting to bootsel mode!");

            // Send notification to display before rebooting
            let mut bootsel_text = heapless::String::<64>::new();
            use core::fmt::Write;
            write!(&mut bootsel_text, "BOOTSEL REBOOT").unwrap();
            display_sender.send(bootsel_text).await;

            // Small delay to let display update
            Timer::after_millis(100).await;

            // Reboot to bootsel mode
            embassy_rp::rom_data::reset_to_usb_boot(0, 0);
        }

        // Format and send display update based on current mode
        let display_text = match current_mode {
            Mode::Numpad => crate::modes::numpad::format_display(&pressed_keys),
            Mode::Calculator => {
                // TODO: Implement calculator display formatting
                let mut text = heapless::String::<64>::new();
                use core::fmt::Write;
                write!(&mut text, "[CALC] TODO").unwrap();
                text
            }
            Mode::M2 | Mode::M3 => {
                let mut text = heapless::String::<64>::new();
                use core::fmt::Write;
                write!(&mut text, "{} Reserved", current_mode.name()).unwrap();
                text
            }
        };

        display_sender.send(display_text).await;

        // Scan rate: 1ms between scans
        Timer::after_millis(1).await;
    }
}
