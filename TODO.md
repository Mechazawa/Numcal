# NumCal TODO

## Current Status
✅ **Working**: USB HID keyboard with OLED display
- Device appears as "NumCal Keyboard" when plugged in
- Sends keypresses to computer via USB HID
- Display shows matrix positions (e.g., "R2C0 R3C1")
- 10ms software debouncing
- 4x6 matrix (4 columns, 6 rows)
- Up to 6 simultaneous keypresses (NKRO)

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
**Status**: ❌ Not implemented yet

**Mode Switching**:
- Hold **Numlock** (R1C0) + press Row 0 keys:
  - **R0C0** → Numpad mode (default)
  - **R0C1** → Calculator mode
  - **R0C2** → Reserved for future
  - **R0C3** → Reserved for future

**Modes**:
1. **Numpad Mode** (default)
   - Sends USB HID keycodes to computer
   - Display shows: `[NUM] R2C0 R3C1`

2. **Calculator Mode**
   - Keys NOT sent to computer (display only)
   - **Numlock** acts as Clear/Reset button:
     - Press when input exists → Clear current input
     - Press when no input → Reset calculator (clear result)
   - Display shows calculator state (see Calculator Mode section below)

3. **Reserved Modes** (M2, M3)
   - Placeholders for future functionality
   - Should be easy to extend

**Display Format**:
- Always show current mode indicator: `[NUM]`, `[CALC]`, `[M2]`, or `[M3]`
- Show pressed keys (excluding Numlock and Row 0 mode switch keys)
- Examples:
  - `[NUM] No keys` - Numpad mode, nothing pressed
  - `[NUM] R2C0 R3C1` - Numpad mode with keys
  - `[CALC] R2C0` - Calculator mode with key

### 3. Key Filtering
- Numlock (R1C0) should not appear as a "pressed key" in display
- Row 0 keys should not appear as "pressed keys" in display
- These keys are only for mode switching

## Technical Notes

### Issues Resolved
1. **USB HID Implementation Crash** - ✅ FIXED
   - **Root cause**: USB logger task (embassy-usb-logger) was conflicting with USB HID
   - **Solution**: Removed USB logger and switched to defmt-only logging via RTT
   - Display now works perfectly alongside USB HID!

### Next Steps
1. **Implement mode switching** (Goal #2)
   - Add Mode enum (Numpad, Calculator, M2, M3)
   - Implement mode switching logic (hold Numlock + Row 0 keys)
   - Update display to show mode indicator (`[NUM]`, `[CALC]`, etc.)
   - Filter USB output based on mode (only send in Numpad mode)
   - Filter display output to hide Numlock and Row 0 keys

2. **Test mode switching thoroughly**
   - Verify mode changes work
   - Verify USB only sends in Numpad mode
   - Verify display shows correct mode indicator

### Architecture
- **keyboard_task**: Scans matrix, detects keys, sends to USB and display channels
- **usb_hid_task**: Receives key events, sends USB HID reports
- **usb_device_task**: Manages USB device enumeration
- **display_task**: Receives text updates, renders to OLED

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
