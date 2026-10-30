use core::cell::RefCell;
use embassy_rp::rom_data::reset_to_usb_boot;
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Instant};
use embassy_usb_logger::ReceiverHandler;
use heapless::{String, Vec};

pub const MAX_SESSIONS: usize = 16;

/// A session that sends nothing for this long is treated as closed, unless it waits for the user.
const SESSION_EXPIRY: Duration = Duration::from_secs(2 * 60 * 60);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Activity {
    Ready,
    Busy,
    Waiting,
    Done,
}

#[derive(Clone, Debug)]
pub struct Session {
    id: String<8>,
    pub activity: Activity,
    pub project: String<14>,
    pub detail: String<14>,
    pub since: Instant,
    pub turn_started: Instant,
    updated: Instant,
}

#[derive(Clone, Debug)]
pub struct Sessions(Vec<Session, MAX_SESSIONS>);

impl Sessions {
    const fn new() -> Self {
        Self(Vec::new())
    }

    pub fn iter(&self) -> impl Iterator<Item = &Session> {
        self.0.iter()
    }

    /// The session to show in full: the one waiting longest for the user, else the latest busy one,
    /// else the latest one heard from.
    pub fn focus(&self) -> Option<&Session> {
        self.iter()
            .filter(|session| session.activity == Activity::Waiting)
            .min_by_key(|session| session.since)
            .or_else(|| {
                self.iter()
                    .filter(|session| session.activity == Activity::Busy)
                    .max_by_key(|session| session.updated)
            })
            .or_else(|| self.iter().max_by_key(|session| session.updated))
    }

    /// Applies one `session\tactivity\tproject\tdetail` line sent by the host hook script.
    fn apply(&mut self, line: &str) {
        let now = Instant::now();
        let mut fields = line.split('\t');
        let id: String<8> = truncated(fields.next().unwrap_or_default());

        self.0
            .retain(|session| session.activity == Activity::Waiting || now - session.updated < SESSION_EXPIRY);

        let activity = match fields.next() {
            Some("ready") => Activity::Ready,
            Some("busy") => Activity::Busy,
            Some("wait") => Activity::Waiting,
            Some("done") => Activity::Done,
            Some("off") => {
                self.0.retain(|session| session.id != id);

                return;
            }
            _ => return,
        };

        let index = self.0.iter().position(|session| session.id == id).unwrap_or_else(|| self.insert(id, now));
        let session = &mut self.0[index];

        if activity == Activity::Busy && !matches!(session.activity, Activity::Busy | Activity::Waiting) {
            session.turn_started = now;
        }

        if activity != session.activity {
            session.since = now;
        }

        session.activity = activity;
        session.project = truncated(fields.next().unwrap_or_default());
        session.detail = truncated(fields.next().unwrap_or_default());
        session.updated = now;
    }

    /// Adds a session, evicting the stalest one that does not wait for the user when the table is full.
    fn insert(&mut self, id: String<8>, now: Instant) -> usize {
        if self.0.is_full() {
            let stalest = self
                .iter()
                .enumerate()
                .filter(|(_, session)| session.activity != Activity::Waiting)
                .min_by_key(|(_, session)| session.updated)
                .map_or(0, |(index, _)| index);

            self.0.remove(stalest);
        }

        let _ = self.0.push(Session {
            id,
            activity: Activity::Ready,
            project: String::new(),
            detail: String::new(),
            since: now,
            turn_started: now,
            updated: now,
        });

        self.0.len() - 1
    }
}

/// Keeps ASCII only, since the display font has no other glyphs and byte slicing stays safe.
fn truncated<const N: usize>(text: &str) -> String<N> {
    text.chars().filter(char::is_ascii).take(N).collect()
}

pub static CLAUDE_SESSIONS: Mutex<CriticalSectionRawMutex, RefCell<Sessions>> =
    Mutex::new(RefCell::new(Sessions::new()));
pub static CLAUDE_CHANGED: Signal<CriticalSectionRawMutex, ()> = Signal::new();

pub fn claude_sessions() -> Sessions {
    CLAUDE_SESSIONS.lock(|sessions| sessions.borrow().clone())
}

/// Collects serial bytes from the host into lines. `bootsel` reboots for flashing; any other line
/// updates [`CLAUDE_SESSIONS`].
pub struct ClaudeLink {
    line: Mutex<CriticalSectionRawMutex, RefCell<Vec<u8, 96>>>,
}

impl ReceiverHandler for ClaudeLink {
    async fn handle_data(&self, data: &[u8]) {
        self.line.lock(|line| {
            let mut line = line.borrow_mut();

            for &byte in data {
                match byte {
                    b'\n' => {
                        match core::str::from_utf8(&line).map(|text| text.trim_end_matches('\r')) {
                            Ok("bootsel") => reset_to_usb_boot(0, 0),
                            #[cfg(feature = "debug-link")]
                            Ok(text) if let Some(command) = crate::debug_link::DebugCommand::parse(text) => {
                                let _ = crate::debug_link::DEBUG_COMMANDS.try_send(command);
                            }
                            Ok(text) => {
                                CLAUDE_SESSIONS.lock(|sessions| sessions.borrow_mut().apply(text));
                                CLAUDE_CHANGED.signal(());
                            }
                            Err(_) => {}
                        }

                        line.clear();
                    }
                    // An overlong line is garbage; drop it and resync on the next newline.
                    _ => {
                        if line.push(byte).is_err() {
                            line.clear();
                        }
                    }
                }
            }
        });
    }

    fn new() -> Self {
        Self {
            line: Mutex::new(RefCell::new(Vec::new())),
        }
    }
}
