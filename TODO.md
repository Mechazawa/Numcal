# NumCal TODO

## Current Status
✅ **Working**: Keyboard matrix scanning with OLED display showing pressed keys
- Display shows matrix positions (e.g., "R2C0 R3C1")
- 10ms software debouncing
- 4x6 matrix (4 columns, 6 rows)

## Goals

### 1. USB HID Keyboard Functionality
**Status**: ⚠️ Attempted but caused display to go black - needs debugging

**Requirements**:
- Device appears as "NumCal Keyboard" when plugged in
- Sends actual keypresses to computer via USB HID
- Only sends keys in Numpad mode (not in Calculator mode)

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
   - Keys displayed on OLED only, NOT sent to computer
   - Display shows: `[CALC] R2C0 R3C1`
   - Future: Implement calculator functionality

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

### Issues Encountered
1. **USB HID Implementation Crash**:
   - When USB HID code was added, display went completely black
   - Possible causes:
     - USB device initialization interfering with logger_task
     - Memory/stack overflow from additional tasks
     - Channel/async issue between tasks
   - Need to debug carefully, possibly one component at a time

### Implementation Strategy (Next Steps)
1. First, get USB HID working WITHOUT mode switching
   - Remove logger_task to free up USB peripheral
   - Use defmt for logging (via RTT with debug probe)
   - Test that keys send to computer

2. Then add mode switching
   - Add Mode enum
   - Implement mode switching logic
   - Update display to show mode
   - Filter keys based on mode

3. Test thoroughly at each step
   - Verify display still works after each change
   - Test mode switching
   - Test USB sending in different modes

### Architecture
- **keyboard_task**: Scans matrix, detects keys, sends to USB and display channels
- **usb_hid_task**: Receives key events, sends USB HID reports
- **usb_device_task**: Manages USB device enumeration
- **display_task**: Receives text updates, renders to OLED

## Future Enhancements
- [ ] Calculator mode: Implement actual calculator functionality
