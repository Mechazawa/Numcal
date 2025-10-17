# NumCal TODO

## Current Status
✅ **Working**: USB HID keyboard with mode switching and OLED display
- Device appears as "NumCal Keyboard" when plugged in
- **Mode switching**: Hold Numlock + Row 0 keys to switch between modes
- **Numpad mode**: Sends keypresses to computer via USB HID
- **Calculator/M2/M3 modes**: Display only (no USB output)
- Display shows mode indicator and filtered key presses
- 10ms software debouncing
- 4x6 matrix (4 columns, 6 rows)
- Up to 6 simultaneous keypresses (NKRO)
- Modular code structure (modes/, keyboard.rs, display.rs, usb.rs)

## Goals

### 1. USB HID Keyboard Functionality
**Status**: ✅ **COMPLETE** - Working!

**What was implemented**:
- Device appears as "NumCal Keyboard" when plugged in ✅
- Sends actual keypresses to computer via USB HID ✅
- Replaced USB logger with defmt-only logging (RTT)
- Currently sends ALL keys (mode switching not yet implemented)

**Keymap** (USB HID keycodes):
- Row 0: Special function keys (reserved for mode switching)
- Row 1: Numlock (0x53), / (0x54), * (0x55), - (0x56)
- Row 2: 7 (0x5F), 8 (0x60), 9 (0x61), nc
- Row 3: 4 (0x5C), 5 (0x5D), 6 (0x5E), + (0x57)
- Row 4: 1 (0x59), 2 (0x5A), 3 (0x5B), nc
- Row 5: nc, 0 (0x62), . (0x63), Enter (0x58)

### 2. Mode Switching System
**Status**: ✅ **COMPLETE** - Working!

**What was implemented**:
- Mode switching via Numlock (R1C0) + Row 0 keys ✅
- Four modes: Numpad (default), Calculator, M2, M3 ✅
- USB output filtered: only Numpad mode sends keys to computer ✅
- Display shows mode indicator: `[NUM]`, `[CALC]`, `[M2]`, `[M3]` ✅
- Display filters out Numlock and Row 0 keys in Numpad mode ✅
- Modular code structure for easy mode extension ✅

**Mode Switching**:
- Hold **Numlock** (R1C0) + press Row 0 keys:
  - **R0C0** → Numpad mode (default)
  - **R0C1** → Calculator mode
  - **R0C2** → M2 (reserved for future)
  - **R0C3** → M3 (reserved for future)

**Modes**:
1. **Numpad Mode** (default) - ✅ Working
   - Sends USB HID keycodes to computer
   - Display shows: `[NUM] R2C0 R3C1` (filtered)

2. **Calculator Mode** - ⚠️ Display only, calculator logic not yet implemented
   - Keys NOT sent to computer (display only)
   - Display shows: `[CALC] TODO`
   - **Numlock** will act as Clear/Reset button (to be implemented)

3. **Reserved Modes** (M2, M3) - ✅ Working
   - Placeholders for future functionality
   - Display shows: `[M2] Reserved` or `[M3] Reserved`

**Display Format**:
- Always shows current mode indicator ✅
- Filters out Numlock and Row 0 keys in Numpad mode ✅
- Examples:
  - `[NUM] No keys` - Numpad mode, nothing pressed
  - `[NUM] R2C0 R3C1` - Numpad mode with keys
  - `[CALC] TODO` - Calculator mode (logic pending)

## Technical Notes

### Issues Resolved
1. **USB HID Implementation Crash** - ✅ FIXED
   - **Root cause**: USB logger task (embassy-usb-logger) was conflicting with USB HID
   - **Solution**: Removed USB logger and switched to defmt-only logging via RTT
   - Display now works perfectly alongside USB HID!

### Next Steps
1. **Implement Calculator Mode functionality** (Goal #3 - see Future Enhancements section)
   - Implement fixed-point arithmetic engine
   - Build expression parser/evaluator
   - Implement Numlock as Clear/Reset button in Calculator mode
   - Create multi-line display layout (input, result, history)
   - Handle errors (div/0, overflow, invalid input)

### Architecture

**File structure**:
- `src/main.rs` - Main entry point, hardware init, task spawning
- `src/modes/mod.rs` - Mode enum and mode switching logic
- `src/modes/numpad.rs` - Numpad mode implementation
- `src/modes/calculator.rs` - Calculator mode implementation (fixed-point arithmetic)
- `src/keyboard.rs` - Keyboard matrix scanning task
- `src/display.rs` - Display rendering task
- `src/usb.rs` - USB HID tasks (device + HID)

**Task architecture**:
- **keyboard_task**: Scans matrix, detects keys, handles mode switching, routes events
- **usb_hid_task**: Receives key events, sends USB HID reports (numpad mode only)
- **usb_device_task**: Manages USB device enumeration
- **display_task**: Receives display updates, renders to OLED (mode-specific content)

## Future Enhancements

### Calculator Mode Implementation
**Status**: ❌ Not started

**Requirements**:
- Must be **accurate** - NO floating point arithmetic
- Use fixed-point arithmetic or rational numbers (fraction representation)
- Support basic operations: +, -, *, /
- Division must maintain precision (e.g., 1/3 = 0.333... displayed accurately)
- Handle operator precedence correctly
- Display input expression and result on OLED
- **Numlock (R1C0)** acts as Clear/Reset:
  - If input exists: Clear current input
  - If no input: Reset calculator (clear result/history)
- Enter key (R5C3) to evaluate expression

**Key mapping in Calculator mode**:
- Row 1: Clear (Numlock), /, *, -
- Row 2: 7, 8, 9, (unused)
- Row 3: 4, 5, 6, +
- Row 4: 1, 2, 3, (unused)
- Row 5: (unused), 0, ., Enter (=)

**Arithmetic approach**:
- **Use fixed-point arithmetic** (decided - best for finance/money calculations)
- Precision: Q16.16 or Q32.32 format (enough for typical financial calculations)
- Consider `fixed` crate or implement custom fixed-point type
- Must handle: addition, subtraction, multiplication, division
- Display with appropriate decimal places (2-4 decimals typical)
- Build expression parser/evaluator for calculator logic
- Future enhancement: Add rational arithmetic mode for exact fractions

**Display layout** (128x64 OLED, ~21 chars per line with FONT_6X10):
- **Line 1**: Mode indicator `[CALC]` + current input expression
  - Example: `[CALC] 123+45`
- **Line 2**: Result or previous calculation
  - Example: `= 168`
- **Line 3-4** (optional): Calculation history if space permits
  - Example: `45-12 = 33`
  - Show last 1-2 calculations

**Display examples**:
```
[CALC] 123+45      <- Line 1: Current input
= 168              <- Line 2: Result
45-12 = 33         <- Line 3: History (optional)
```

```
[CALC] 1/3         <- Line 1: Current input
= 0.333...         <- Line 2: Result (repeating decimal)
```

```
[CALC]             <- Line 1: No input
= 0                <- Line 2: Ready state
```

**Error handling**:
- Division by zero: Display "Error: Div/0"
- Overflow: Display "Error: Overflow"
- Invalid input: Display "Error: Invalid"
