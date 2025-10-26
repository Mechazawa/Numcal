// Numpad mode implementation
// In this mode, keys are sent via USB HID to the computer

use crate::KeyEvent;

/// Process a key event in Numpad mode
/// Returns Some(event) if the key should be sent to USB, None otherwise
#[allow(dead_code)]
pub fn handle_key_event(event: KeyEvent) -> Option<KeyEvent> {
    // In numpad mode, send all non-zero keycodes to USB
    // Row 0 keys (mode switching) are already filtered out by having 0x00 in keymap
    Some(event)
}

/// Format display text for Numpad mode
/// Filters out mode switching keys (Numlock and Row 0)
pub fn format_display(pressed_keys: &[(usize, usize)]) -> heapless::String<64> {
    use core::fmt::Write;

    let mut text = heapless::String::<64>::new();
    write!(&mut text, "[NUM] ").unwrap();

    // Filter out Numlock (R1C0) and Row 0 keys
    let filtered_keys: heapless::Vec<(usize, usize), 24> = pressed_keys
        .iter()
        .filter(|(row, col)| *row != 0 && !(*row == 1 && *col == 0))
        .copied()
        .collect();

    if filtered_keys.is_empty() {
        write!(&mut text, "No keys").unwrap();
    } else {
        for (i, (row, col)) in filtered_keys.iter().enumerate() {
            if i > 0 {
                write!(&mut text, " ").unwrap();
            }
            write!(&mut text, "R{row}C{col}").unwrap();
        }
    }

    text
}
