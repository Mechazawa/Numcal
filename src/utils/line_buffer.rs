use heapless::Vec;

/// Assembles newline-terminated text lines from serial packets.
pub struct LineBuffer(Vec<u8, 256>);

impl LineBuffer {
    pub const fn new() -> Self {
        Self(Vec::new())
    }

    pub fn feed(&mut self, data: &[u8], mut on_line: impl FnMut(&str)) {
        for &byte in data {
            if byte != b'\n' {
                // An overlong line is garbage; drop it and resync on the next newline.
                if self.0.push(byte).is_err() {
                    self.0.clear();
                }

                continue;
            }

            if let Ok(line) = core::str::from_utf8(&self.0) {
                on_line(line.trim_end_matches('\r'));
            }

            self.0.clear();
        }
    }
}
