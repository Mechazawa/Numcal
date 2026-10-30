# Agent notes

## Display
- The OLED panel shows only a 128x32 window (x 4..131, y 32..63) of the 132x64 SSD1306 buffer. `DisplayProxy` handles that offset: draw in `0..DisplayProxy::WIDTH` x `0..DisplayProxy::HEIGHT` and anything outside is clipped.
- The display channel has 64 slots and `DisplayProxy` uses `try_send(...).unwrap()`, so an overflowing frame panics and hangs the device. Call `display.idle().await` before drawing a frame in an animation loop.

## Flashing
- `cargo run --release` flashes without touching the device: `flash.sh` sends `bootsel` over the USB serial port and the firmware reboots into BOOTSEL. `picotool reboot` does not work with this firmware.
- If the firmware hangs and drops off USB, the device needs a replug (or F1+F2+F3+F4 while it still runs).

## Debugging without looking at the device
- Build with `cargo run --release --features debug-link`, then use `scripts/numcal-debug.py`:
  - `dump [--png out.png]` prints the visible screen as text from a mirror of the display buffer.
  - `key 1 Sub Enter`, `down Lock`, `up Lock` inject key events (`down Lock`, `key F3`, `up Lock` switches to the Claude mode).
  - `send "<line>"` sends a raw serial line, such as a fake hook status.
- Injected Claude-mode keys type into the focused window on the Mac, so do not inject them while a Claude Code terminal has focus.

## Claude Code mode (Lock+F3)
- `claude/numcal-hook.sh` is a Claude Code hook. It sends `session\tactivity\tproject\tdetail` lines to the device; `src/tasks/claude_link.rs` parses them. It is installed in `~/.claude/settings.json` for SessionStart, SessionEnd, UserPromptSubmit, PreToolUse, PostToolUse, PostToolUseFailure, PermissionRequest, Elicitation, Stop, StopFailure and Notification (`idle_prompt`).
- `Stop` does not fire when the user interrupts a turn; the `idle_prompt` notification marks such a session idle later.

## Lint
- `cargo clippy --release -- -W clippy::pedantic` and the same with `--features debug-link` must be clean.
