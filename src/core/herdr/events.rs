//! Follows `hc events` for this app's terminals, so a pane whose shell
//! changed folder is published the moment herdr-core says so, rather than
//! at the next tick: the Files and Changes panels follow that folder.
//!
//! A thread of its own runs `hc events --json --label moonframe-app=<NAME>`
//! and gives each `foreground` event's terminal and folder to [`Live`].
//! When the stream ends (the server restarted, `hc` failed) it starts again
//! after a pause that grows to [`MAX_PAUSE`], and what it heard is
//! forgotten: the listings say where each shell is until it follows
//! again. Its output is untrusted: each line is read up to [`MAX_LINE`]
//! bytes, and only an event's type, terminal and folder are looked at.

use {
    super::super::{live::Live, new_command},
    serde_json::Value,
    std::{
        io::{self, BufRead, BufReader},
        path::Path,
        process::{Child, Stdio},
        sync::{
            Arc, Mutex, PoisonError,
            atomic::{AtomicBool, Ordering},
        },
        thread,
        time::Duration,
    },
};

/// Bytes of one event line looked at; the rest of a longer one is skipped.
const MAX_LINE: usize = 64 * 1024;
/// The pause before following again after the stream ended, at first and
/// at most.
const FIRST_PAUSE: Duration = Duration::from_millis(500);
const MAX_PAUSE: Duration = Duration::from_secs(30);

/// What the thread and its owner share.
#[derive(Default)]
struct Shared {
    /// Set when the owner is gone: the thread ends.
    stop: AtomicBool,
    /// The running `hc events`, so the owner can end it.
    child: Mutex<Option<Child>>,
}

/// The follower: dropping it ends `hc events` and its thread.
pub(super) struct Events {
    shared: Arc<Shared>,
}

impl Events {
    /// Follows the events of this app's terminals through `hc`, telling
    /// `live` where their shells move to.
    pub(super) fn follow(hc: &Path, live: Live) -> Self {
        let shared = Arc::new(Shared::default());
        let thread_shared = Arc::clone(&shared);
        let hc = hc.to_owned();
        let label = format!("{}={}", super::APP_LABEL, crate::NAME);
        let spawned = thread::Builder::new()
            .name("hc-events".to_owned())
            .spawn(move || run(&hc, &label, &live, &thread_shared));
        if let Err(error) = spawned {
            tracing::warn!("core: cannot follow hc events: {error}");
        }
        Self { shared }
    }
}

impl Drop for Events {
    fn drop(&mut self) {
        self.shared.stop.store(true, Ordering::Release);
        end(&self.shared);
    }
}

/// Ends the running `hc events`, if any.
fn end(shared: &Shared) {
    let child = shared
        .child
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .take();
    if let Some(mut child) = child {
        let _ = child.kill();
        let _ = child.wait();
    }
}

/// Follows `hc events` until the owner is gone, starting it again when it
/// ends.
fn run(hc: &Path, label: &str, live: &Live, shared: &Shared) {
    let mut pause = FIRST_PAUSE;
    while !shared.stop.load(Ordering::Acquire) {
        match follow_once(hc, label, live, shared) {
            // It ran a while: a server restart, not a broken `hc`.
            Ok(true) => pause = FIRST_PAUSE,
            Ok(false) => {}
            Err(error) => tracing::debug!("core: hc events: {error}"),
        }
        end(shared);
        // Moves made while it is not running go unheard.
        live.forget();
        sleep_unless_stopped(pause, shared);
        pause = (pause * 2).min(MAX_PAUSE);
    }
}

/// Runs `hc events` once, telling `live` of each move, until its output
/// ends; whether it said anything.
fn follow_once(hc: &Path, label: &str, live: &Live, shared: &Shared) -> io::Result<bool> {
    let mut command = new_command(hc);
    command
        .args(["events", "--json", "--label", label])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let mut child = command.spawn()?;
    let stdout = child.stdout.take();
    {
        let mut slot = shared.child.lock().unwrap_or_else(PoisonError::into_inner);
        if shared.stop.load(Ordering::Acquire) {
            let _ = child.kill();
            let _ = child.wait();
            return Ok(false);
        }
        *slot = Some(child);
    }
    let Some(stdout) = stdout else {
        return Ok(false);
    };
    let mut reader = BufReader::new(stdout);
    let mut line = Vec::new();
    let mut heard = false;
    while read_line(&mut reader, &mut line, MAX_LINE)? {
        heard = true;
        if let Some((terminal, cwd)) = moved_to(&line) {
            live.moved(&terminal, &cwd);
        }
    }
    Ok(heard)
}

/// Waits `pause`, waking early when the owner is gone.
fn sleep_unless_stopped(pause: Duration, shared: &Shared) {
    let step = Duration::from_millis(100);
    let mut waited = Duration::ZERO;
    while waited < pause && !shared.stop.load(Ordering::Acquire) {
        thread::sleep(step);
        waited += step;
    }
}

/// Reads one line of `reader` into `line`, keeping at most `max` bytes of
/// it and skipping the rest; `false` at the end of the output.
fn read_line(reader: &mut impl BufRead, line: &mut Vec<u8>, max: usize) -> io::Result<bool> {
    line.clear();
    let mut any = false;
    loop {
        let buffer = reader.fill_buf()?;
        if buffer.is_empty() {
            return Ok(any);
        }
        any = true;
        let (part, done) = match buffer.iter().position(|&byte| byte == b'\n') {
            Some(at) => (&buffer[..at], at + 1),
            None => (buffer, buffer.len()),
        };
        let room = max.saturating_sub(line.len());
        line.extend_from_slice(&part[..part.len().min(room)]);
        let ended = done > part.len();
        reader.consume(done);
        if ended {
            return Ok(true);
        }
    }
}

/// The terminal and folder of a `foreground` event, which herdr-core sends
/// when a terminal's foreground program changes and when its shell changes
/// folder: `{ "method": "event", "params": { "type": "foreground",
/// "terminal": "t7", "data": { "cwd": "/home/u/code", ... } } }`.
fn moved_to(line: &[u8]) -> Option<(String, String)> {
    let event: Value = serde_json::from_slice(line).ok()?;
    let params = event.get("params")?;
    if params.get("type").and_then(Value::as_str) != Some("foreground") {
        return None;
    }
    let terminal = params.get("terminal").and_then(Value::as_str)?;
    let cwd = params.pointer("/data/cwd").and_then(Value::as_str)?;
    Some((terminal.to_owned(), cwd.to_owned()))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn a_folder_change_is_a_foreground_event() {
        // As `hc events --json` printed it after `cd /tmp`.
        let moved = br#"{"jsonrpc":"2.0","method":"event","params":{"data":{"argv":["/bin/zsh","-l"],"cwd":"/private/tmp","name":"zsh","pid":1},"seq":331,"terminal":"t2m","type":"foreground"}}"#;
        assert_eq!(
            moved_to(moved),
            Some(("t2m".to_owned(), "/private/tmp".to_owned()))
        );
        // The listing it starts with, another event, one missing its
        // folder, and noise are not.
        assert_eq!(
            moved_to(br#"{"channel":null,"seq":329,"terminals":[]}"#),
            None
        );
        assert_eq!(
            moved_to(br#"{"method":"event","params":{"type":"status","terminal":"t1","data":{"cwd":"/"}}}"#),
            None
        );
        assert_eq!(
            moved_to(
                br#"{"method":"event","params":{"type":"foreground","terminal":"t1","data":{}}}"#
            ),
            None
        );
        assert_eq!(moved_to(b"not json"), None);
        assert_eq!(moved_to(b""), None);
    }

    #[test]
    fn a_long_line_is_cut_and_the_next_one_still_read() {
        let mut input = vec![b'x'; 100];
        input.extend_from_slice(b"\nshort\nlast");
        let mut reader = BufReader::with_capacity(16, input.as_slice());
        let mut line = Vec::new();
        assert!(read_line(&mut reader, &mut line, 10).unwrap());
        assert_eq!(line, vec![b'x'; 10]);
        assert!(read_line(&mut reader, &mut line, 10).unwrap());
        assert_eq!(line, b"short");
        // A last line without a newline is still a line.
        assert!(read_line(&mut reader, &mut line, 10).unwrap());
        assert_eq!(line, b"last");
        assert!(!read_line(&mut reader, &mut line, 10).unwrap());
    }

    #[cfg(unix)]
    #[test]
    fn a_move_from_hc_is_published_and_dropping_ends_it() {
        use {
            moonframe::{Data, Handle},
            serde_json::json,
            std::{os::unix::fs::PermissionsExt as _, sync::atomic::AtomicUsize, time::Instant},
        };

        let dir = crate::git::tests::tempdir();
        let hc = dir.join("hc");
        // An `hc` whose events say t1's shell went to /srv, then stay open.
        std::fs::write(
            &hc,
            "#!/bin/sh\necho '{\"method\":\"event\",\"params\":{\"type\":\"foreground\",\"terminal\":\"t1\",\"data\":{\"cwd\":\"/srv\"}}}'\nexec sleep 60\n",
        )
        .unwrap();
        std::fs::set_permissions(&hc, std::fs::Permissions::from_mode(0o755)).unwrap();
        let published = Arc::new(AtomicUsize::new(0));
        let count = Arc::clone(&published);
        let host = Handle::new(
            "core",
            &dir,
            Data::Nil,
            Arc::new(move |_| {
                count.fetch_add(1, Ordering::SeqCst);
            }),
        );
        let live = Live::new(&host, "/home/u");
        live.publish(json!({ "terminals": [{ "address": "t1", "cwd": "~" }] }));

        let events = Events::follow(&hc, live);
        let started = Instant::now();
        while published.load(Ordering::SeqCst) < 2 {
            assert!(
                started.elapsed() < Duration::from_secs(5),
                "the move was not published"
            );
            thread::sleep(Duration::from_millis(10));
        }
        let shared = Arc::clone(&events.shared);
        drop(events);
        assert!(shared.stop.load(Ordering::Acquire));
        assert!(
            shared.child.lock().unwrap().is_none(),
            "hc events was ended"
        );
    }
}
