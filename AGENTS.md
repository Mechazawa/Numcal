# Agent notes

## Display
- The OLED panel shows only a 128x32 window (x 4..131, y 32..63) of the 132x64 SSD1306 buffer. `DisplayProxy` handles that offset: draw in `0..DisplayProxy::WIDTH` x `0..DisplayProxy::HEIGHT` and anything outside is clipped.
- The display channel has 64 slots and `DisplayProxy` uses `try_send(...).unwrap()`, so an overflowing frame panics and hangs the device. Call `display.idle().await` before drawing a frame in an animation loop.

## USB
- One USB device with two serial ports and a keyboard: `/dev/cu.usbmodem123456781` carries logs, `bootsel` and debug commands; `/dev/cu.usbmodem123456783` carries Claude Code traffic. Five interfaces need the embassy-usb `max-interface-count-8` feature; without it USB setup panics.

## Flashing
- `cargo run --release` flashes without touching the device: `flash.sh` sends `bootsel` on the log port and the firmware reboots into BOOTSEL. `picotool reboot` does not work with this firmware.
- A panic reboots into BOOTSEL, and holding F1 while plugging in does too, so a broken build can always be reflashed without opening the case.

## Debugging without looking at the device
- Build with `cargo run --release --features debug-link`, then use `scripts/numcal-debug.py`:
  - `dump [--png out.png]` prints the visible screen as text from a mirror of the display buffer.
  - `key 1 Sub Enter`, `down Lock`, `up Lock` inject key events (`down Lock`, `key F3`, `up Lock` switches to the Claude mode).
  - `send "<line>"` sends a raw serial line, such as a fake hook status.
- Injected Claude-mode keys type into the focused window on the Mac, so do not inject them while a Claude Code terminal has focus.

## Claude Code mode (Lock+F3)
- `claude/numcal-hook.sh` is a Claude Code hook. It sends `session\tactivity\tproject\tdetail` lines to the Claude port; `src/tasks/claude_link.rs` parses them. It is installed in `~/.claude/settings.json` for SessionStart, SessionEnd, UserPromptSubmit, PreToolUse, PostToolUse, PostToolUseFailure, PermissionRequest, Elicitation, Stop, StopFailure and Notification (`idle_prompt`).
- `claude/numcal-helper.py` (launchd agent `claude/nl.mechazawa.numcal-helper.plist`) sends a `helper` heartbeat and turns the device's `keys`/`jump` requests into Ghostty AppleScript calls. It finds a session's terminal by the terminal's foreground `pid` and `~/.claude/sessions/<pid>.json`; tty paths and folders are reused, so they are not used for matching. Needs Ghostty 1.4+, whose `send key` takes camelCase names such as `digit1` and `arrowDown`.
- Without a heartbeat in the last 5 seconds the mode types over USB into the focused window instead. Lock tapped alone requests a jump to the session's terminal.
- `NUMCAL_SESSIONS_DIR` points the helper at fake session records, so a throwaway `cat` tab can stand in for a session during tests.
- `Stop` does not fire when the user interrupts a turn; the `idle_prompt` notification marks such a session idle later.

## Canvas (Lock+F4)
- Agents draw with the `numcal` CLI (`claude/numcal`, symlinked to `~/.local/bin/numcal`) and the `numcal-display` skill. The helper relays CLI requests from `~/.cache/numcal/helper.sock` to the Claude port as `canvas\t<op>` lines and hands `key\t<name>` events back to waiting `numcal wait-key`/`ask` calls.
- Any canvas line switches the device to page 4. Ops render into a local framebuffer, so `invert` works and a frame fits in the display channel whatever its op count (max 40 ops per frame).
- Pages 3 and 4 never type over USB: page 3 sends keys to the session's terminal through the helper, page 4 sends key events to agents.

## Lint
- `cargo clippy --release -- -W clippy::pedantic` and the same with `--features debug-link` must be clean.
