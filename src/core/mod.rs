//! `app.core`: the terminals this app's backends run, and what the app asks
//! of them.
//!
//! A backend keeps terminals running when the window closes: herdr-core
//! (`hc`, herdr.rs) or tmux (tmux/). Each one compiled in (the cargo
//! features `herdr-core` and `tmux`) and installed is listed each tick, and
//! what they list is published together: an agent in one of this app's
//! panes shows in its pane, and one started anywhere else (another app,
//! `hc run`, the user's own tmux) shows too, to be peeked at. A backend that
//! hears a pane's shell change folder between ticks (herdr-core's `hc
//! events`) has it published at once (live.rs).
//!
//! What it publishes:
//!
//! ```text
//! { ok, error, now, core = { name, version },
//!   backends = { auto, list = { { id, installed, ok, error, version } } },
//!   terminals = { { address, target, backend, name, agent, program,
//!                   program_key, cwd, status, status_ms, turn, parent,
//!                   ours, pane, is_agent } },
//!   counts = { live, agents, working, blocked, done },
//!   ended = { "p3", ... },   -- this app's panes whose shell exited
//!   programs = { { key, name, installed } },   -- the agents "+" can start
//!   peek = { address, text, error },
//!   app_version,             -- this app's version
//!   machines = { ok, error, list = { { name, target, state, version } } } }
//! ```
//!
//! `core` is herdr-core's server (`null` without it). `backends.list` holds
//! the backends compiled in, herdr-core first; `auto` is the one a pane set
//! to Auto opens on: herdr-core when `hc` is installed, else tmux (`""`
//! when neither is). `ok` says at least one installed backend could be
//! read; `error` says why one could not.
//!
//! A terminal's `address` is what actions name: herdr-core's own (`t7`) or
//! `server:%id` for a tmux pane (`default:%12`). `target` is what to show
//! the user: herdr-core's address, or tmux's pane (`%3` in one of ours,
//! `work:1.0` elsewhere). `backend` is `herdr-core` or `tmux`. `program`
//! is what to call it (`Claude Code`, `zsh`), and `program_key` the key its
//! icon is drawn by (`claude`, `shell`, or `""` for one the app does not
//! know): see programs.rs. `status_ms` is when it entered its status (ms
//! since the epoch; 0 without one), and `now` the time rounded down to the
//! minute, for the views to count ages by: Luau has no clock. `parent` is
//! the terminal that started it, if any.
//!
//! What it takes: `{ watch = address }` (publish that terminal's screen as
//! `peek` each tick; `""` stops), `{ send = address, text }`,
//! `{ keys = address, key = "enter" }`, `{ interrupt = address }`,
//! `{ close_panes = "p1,p4" }` (end the shells of those panes of this app,
//! on every backend), `{ start = "p3", program = "claude" }` (type a
//! startable agent's command into that pane's new shell, once: the
//! terminal is marked first, and a marked one is never typed into again,
//! across reloads and launches). Actions are answered with
//! `{ reply = <action>, address, ok, error }`.
//!
//! These reach any terminal of any backend, not only this app's: a peek
//! answers agents running elsewhere. So only the mods named in
//! provider.toml's `[settings] writers` may send them, as Moonframe's git
//! provider does; a mod from a package the user installed is refused.
//!
//! Any mod may send two requests that touch no terminal: `{ refresh =
//! "machines" }` (read `hc machine ls` again at once, rather than at the
//! next look at the server) and `{ open = "releases" }` (open this app's
//! releases page in the browser: the only address it opens), answered as
//! `{ reply = "open", address, ok, error }`. Each runs at most once a tick.

#[cfg(not(any(feature = "herdr-core", feature = "tmux")))]
compile_error!("enable at least one terminal backend: the `herdr-core` or `tmux` feature");

#[cfg(feature = "herdr-core")]
mod herdr;
#[cfg(feature = "herdr-core")]
pub(crate) use herdr::find_hc;
mod live;
#[cfg(feature = "tmux")]
mod tmux;

use {
    moonframe::{Data, Handle, Native},
    serde_json::{Value, json},
    std::{
        io::{self, Read, Write as _},
        process::{Command, Output, Stdio},
        thread,
        time::{Duration, Instant},
    },
};

/// Where this app's releases are, for Settings > Updates.
const RELEASES: &str = "https://github.com/moonframehq/moonmux/releases";
/// Ticks between looks for installed agents and backends.
const PROGRAMS_EVERY: u64 = 10;
/// Ticks between looks at herdr-core's machines.
const MACHINES_EVERY: u64 = 30;

/// What one backend listed at a tick.
#[derive(Debug, Default)]
pub(crate) struct Listing {
    /// Its live terminals, as the mods read them.
    pub(crate) terminals: Vec<Value>,
    /// This app's panes whose shell finished on it.
    pub(crate) finished: Vec<String>,
}

/// One way of keeping terminals: what `app.core` lists and acts through.
/// Every action names a terminal by the address the backend listed it
/// under, or this app's panes by their ids.
pub(crate) trait Backend: Send {
    /// What a pane's `backend` prop names it: `herdr-core` or `tmux`.
    fn id(&self) -> &'static str;
    /// Whether its program was found on this machine.
    fn installed(&self) -> bool;
    /// Looks for its program again: one installed while the app runs is
    /// picked up.
    fn find(&mut self);
    /// Its server: at least `version` (`""` when unknown).
    fn info(&self) -> Value;
    /// The live terminals, and this app's panes whose shell finished.
    fn list(&mut self, ticks: u64, home: &str) -> Result<Listing, String>;
    /// Whether `address` is one of its addresses.
    fn owns(&self, address: &str) -> bool;
    /// The screen of `address`, with some history above it.
    fn read(&mut self, address: &str) -> Result<String, String>;
    fn send(&mut self, address: &str, text: &str) -> Result<String, String>;
    fn keys(&mut self, address: &str, key: &str) -> Result<String, String>;
    fn interrupt(&mut self, address: &str) -> Result<String, String>;
    /// Ends the shells of this app's panes `panes` that run on it.
    fn close_panes(&mut self, panes: &[&str]) -> Result<String, String>;
    /// Types `command` into the shell of this app's pane `pane`, once,
    /// marking it as given the agent `key` first. `Ok(None)` when the pane
    /// has no live terminal on this backend.
    fn start(&mut self, pane: &str, key: &str, command: &str) -> Result<Option<String>, String>;
    /// Starts telling `live` where this app's panes' shells move to as it
    /// hears it, between ticks, if it can; called again after each `find`.
    fn follow(&mut self, _live: &live::Live) {}
    /// The machines it reaches (herdr-core's `hc machine ls`), if it has
    /// any notion of them.
    fn machines(&mut self) -> Option<Value> {
        None
    }
}

/// The backends compiled in, herdr-core first: the order Auto picks in.
fn backends() -> Vec<Box<dyn Backend>> {
    vec![
        #[cfg(feature = "herdr-core")]
        Box::new(herdr::HerdrCore::new()),
        #[cfg(feature = "tmux")]
        Box::new(tmux::Tmux::new()),
    ]
}

pub(crate) struct Core {
    backends: Vec<Box<dyn Backend>>,
    /// The terminal shown in a peek, if any.
    watching: String,
    /// The machines herdr-core reaches over ssh, read now and then.
    machines: Value,
    /// The tick of the last refresh and of the last page opened: any mod
    /// may ask for either, so each runs at most once a tick.
    refreshed: Option<u64>,
    opened: Option<u64>,
    /// The startable agents and whether each is installed, looked at now
    /// and then.
    programs: Value,
    ticks: u64,
    /// What was last published, shared with the backends that follow
    /// moves between ticks; from `start` on.
    live: Option<live::Live>,
    /// The mods that may watch and act (`[settings] writers`).
    writers: Vec<String>,
}

impl Core {
    pub(crate) fn new() -> Self {
        Self::with(backends())
    }

    fn with(backends: Vec<Box<dyn Backend>>) -> Self {
        Self {
            backends,
            watching: String::new(),
            machines: Value::Null,
            refreshed: None,
            opened: None,
            programs: Value::Null,
            ticks: 0,
            live: None,
            writers: Vec::new(),
        }
    }

    /// The backend `address` belongs to.
    fn owner(&mut self, address: &str) -> Result<&mut Box<dyn Backend>, String> {
        self.backends
            .iter_mut()
            .find(|backend| backend.owns(address))
            .ok_or_else(|| format!("`{address}` is not a terminal this app can reach"))
    }

    fn snapshot(&mut self) -> Value {
        let every = |period: u64| self.ticks.is_multiple_of(period);
        if every(PROGRAMS_EVERY) || self.programs.is_null() {
            self.programs = crate::programs::installed();
            if self.ticks > 0 {
                for backend in &mut self.backends {
                    backend.find();
                    if let Some(live) = &self.live {
                        backend.follow(live);
                    }
                }
            }
        }
        if every(MACHINES_EVERY) || self.machines.is_null() {
            self.machines = self
                .backends
                .iter_mut()
                .find_map(|backend| backend.machines())
                .unwrap_or_else(|| {
                    json!({ "ok": false, "error": "herdr-core is not part of this build", "list": [] })
                });
        }
        let home = std::env::var("HOME").unwrap_or_default();
        let mut terminals = Vec::new();
        let mut finished = Vec::new();
        let mut list = Vec::new();
        let mut core = Value::Null;
        let mut errors = Vec::new();
        let mut ok = false;
        for backend in &mut self.backends {
            let info = backend.info();
            if backend.id() == "herdr-core" && backend.installed() {
                core = info.clone();
            }
            let listed = backend.list(self.ticks, &home);
            let error = listed.as_ref().err().cloned().unwrap_or_default();
            if !error.is_empty() {
                errors.push((backend.installed(), error.clone()));
            }
            ok |= listed.is_ok();
            if let Ok(listing) = listed {
                terminals.extend(listing.terminals);
                finished.extend(listing.finished);
            }
            list.push(json!({
                "id": backend.id(),
                "installed": backend.installed(),
                "ok": error.is_empty(),
                "error": error,
                "version": info.get("version").cloned().unwrap_or_else(|| json!("")),
            }));
        }
        let auto = self
            .backends
            .iter()
            .find(|backend| backend.installed())
            .map_or("", |backend| backend.id());
        let backends = json!({ "auto": auto, "list": list });
        // A backend that is not installed matters only while none could be
        // read.
        let error = errors
            .iter()
            .filter(|(installed, _)| *installed || !ok)
            .map(|(_, error)| error.as_str())
            .collect::<Vec<_>>()
            .join("; ");
        if !ok {
            // Settings still shows the app's version and why the machines
            // could not be read.
            return json!({
                "ok": false,
                "error": error,
                "terminals": [],
                "counts": counts(&[]),
                "backends": backends,
                "app_version": env!("CARGO_PKG_VERSION"),
                "machines": self.machines,
            });
        }
        let ended = ended_panes(finished, &terminals);
        let mut snapshot = json!({
            "ok": true,
            "error": error,
            "now": to_minute(now_ms()),
            "core": core,
            "backends": backends,
            "counts": counts(&terminals),
            "terminals": terminals,
            "ended": ended,
            "app_version": env!("CARGO_PKG_VERSION"),
            "machines": self.machines,
            "programs": self.programs,
        });
        if !self.watching.is_empty() {
            let watching = self.watching.clone();
            let peek = match self
                .owner(&watching)
                .and_then(|backend| backend.read(&watching))
            {
                Ok(text) => json!({ "address": watching, "text": text, "error": "" }),
                Err(error) => json!({ "address": watching, "text": "", "error": error }),
            };
            snapshot["peek"] = peek;
        }
        snapshot
    }

    /// Publishes what the backends list, unless nothing changed.
    fn publish(&mut self) {
        let snapshot = self.snapshot();
        if let Some(live) = &self.live {
            live.publish(snapshot);
        }
    }

    /// An action a mod asked for: what the backend was told, and whether it
    /// took.
    fn act(&mut self, message: &Data) -> Option<(&'static str, String, Result<String, String>)> {
        let field = |key: &str| message.get(key).and_then(Data::as_str).map(str::to_owned);
        if let Some(address) = field("send") {
            let text = field("text").unwrap_or_default();
            let result = self
                .owner(&address)
                .and_then(|backend| backend.send(&address, &text));
            return Some(("send", address, result));
        }
        if let Some(address) = field("keys") {
            let key = field("key").unwrap_or_else(|| "enter".to_owned());
            let result = self
                .owner(&address)
                .and_then(|backend| backend.keys(&address, &key));
            return Some(("keys", address, result));
        }
        if let Some(address) = field("interrupt") {
            let result = self
                .owner(&address)
                .and_then(|backend| backend.interrupt(&address));
            return Some(("interrupt", address, result));
        }
        if let Some(panes) = field("close_panes") {
            let result = self.close_panes(&panes);
            return Some(("close_panes", panes, result));
        }
        if let Some(pane) = field("start") {
            let result = self.start(&pane, &field("program").unwrap_or_default());
            return Some(("start", pane, result));
        }
        None
    }

    /// Types the startable agent `key`'s command into the new shell of this
    /// app's pane `pane`, on whichever backend runs it, once: the backend
    /// marks the terminal before typing, and leaves a marked one alone, so
    /// neither a message sent twice nor a reload starts a second agent.
    fn start(&mut self, pane: &str, key: &str) -> Result<String, String> {
        let program = crate::programs::startable(key)
            .ok_or_else(|| format!("`{key}` is not an agent this app starts"))?;
        let found = crate::programs::locate(program.run)
            .ok_or_else(|| format!("{} is not installed", program.name))?;
        let path = std::env::var_os("PATH");
        let command = crate::programs::command_line(program, &found, path.as_deref());
        self.start_on_backends(pane, key, &command)
    }

    /// Gives `command` to the backend that runs `pane`. One backend failing
    /// (herdr-core installed but not answering) must not stop the pane from
    /// starting on another: its error only counts when no backend ran it.
    fn start_on_backends(
        &mut self,
        pane: &str,
        key: &str,
        command: &str,
    ) -> Result<String, String> {
        let mut errors = Vec::new();
        for backend in self
            .backends
            .iter_mut()
            .filter(|backend| backend.installed())
        {
            match backend.start(pane, key, command) {
                Ok(Some(done)) => return Ok(done),
                Ok(None) => {}
                Err(error) => errors.push(format!("{}: {error}", backend.id())),
            }
        }
        if errors.is_empty() {
            Err(format!("pane {pane} has no live terminal"))
        } else {
            Err(errors.join("; "))
        }
    }

    /// Ends the shells of this app's panes `panes` (`p1,p4`) on every
    /// backend: closing a pane ends what runs in it, as closing the window
    /// does not.
    fn close_panes(&mut self, panes: &str) -> Result<String, String> {
        let panes: Vec<&str> = panes.split(',').filter(|pane| !pane.is_empty()).collect();
        let problems: Vec<String> = self
            .backends
            .iter_mut()
            .filter(|backend| backend.installed())
            .filter_map(|backend| backend.close_panes(&panes).err())
            .collect();
        if problems.is_empty() {
            Ok(String::new())
        } else {
            Err(problems.join("; "))
        }
    }
}

impl Native for Core {
    fn start(&mut self, host: &Handle) -> Result<(), String> {
        self.writers = writers(host.settings());
        let live = live::Live::new(host, &std::env::var("HOME").unwrap_or_default());
        for backend in &mut self.backends {
            backend.follow(&live);
        }
        self.live = Some(live);
        self.publish();
        Ok(())
    }

    fn tick(&mut self, _host: &Handle) {
        self.ticks += 1;
        self.publish();
    }

    fn receive(&mut self, host: &Handle, from: &str, message: Data) {
        // What touches no terminal, any mod may ask.
        if message.get("refresh").and_then(Data::as_str) == Some("machines") {
            if once_a_tick(&mut self.refreshed, self.ticks) {
                self.machines = Value::Null;
                self.publish();
            }
            return;
        }
        if let Some(page) = message.get("open").and_then(Data::as_str) {
            let result = if once_a_tick(&mut self.opened, self.ticks) {
                open_page(page)
            } else {
                Err("a page was opened a moment ago".to_owned())
            };
            if let Err(error) = &result {
                tracing::warn!("core: open {page}: {error}");
            }
            host.send(from, reply("open", page, result));
            return;
        }
        // Typing into, interrupting or reading another program's terminal
        // is for this app's own mods, never one from elsewhere.
        if !may_act(&self.writers, from) {
            tracing::warn!(
                "core: refused `{from}`: only the mods in providers/core's `writers` may act"
            );
            host.send(
                from,
                Data::map([
                    ("reply", "refused".into()),
                    ("ok", false.into()),
                    (
                        "error",
                        format!("`{from}` may not act on terminals: only the mods in providers/core's `writers` may").into(),
                    ),
                ]),
            );
            return;
        }
        if let Some(address) = message.get("watch").and_then(Data::as_str) {
            self.watching = address.to_owned();
        } else if let Some((action, address, result)) = self.act(&message) {
            if let Err(error) = &result {
                tracing::warn!("core: {action} {address}: {error}");
            }
            host.send(from, reply(action, &address, result));
        }
        self.publish();
    }
}

/// Whether what was last done at tick `last` may run again at `now`, and
/// if so, marks it done now.
fn once_a_tick(last: &mut Option<u64>, now: u64) -> bool {
    if *last == Some(now) {
        return false;
    }
    *last = Some(now);
    true
}

/// The answer to an action: `{ reply = <action>, address, ok, error }`.
fn reply(action: &str, address: &str, result: Result<String, String>) -> Data {
    Data::map([
        ("reply", action.into()),
        ("address", address.into()),
        ("ok", result.is_ok().into()),
        ("error", result.err().unwrap_or_default().into()),
    ])
}

/// The address of a page `{ open = <page> }` may name.
fn page_address(page: &str) -> Option<&'static str> {
    match page {
        "releases" => Some(RELEASES),
        _ => None,
    }
}

/// Opens `page` in the system's browser.
fn open_page(page: &str) -> Result<String, String> {
    let address = page_address(page).ok_or_else(|| format!("no page called {page:?}"))?;
    let opener = if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(windows) {
        "explorer"
    } else {
        "xdg-open"
    };
    let status = new_command(opener)
        .arg(address)
        .status()
        .map_err(|error| format!("{opener}: {error}"))?;
    opened(opener, status.success(), status.code())
}

/// Whether the opener's exit says the page opened. explorer exits with 1
/// even when it opened the page, so on Windows only a start failure counts.
fn opened(opener: &str, success: bool, code: Option<i32>) -> Result<String, String> {
    if success || cfg!(windows) {
        return Ok(String::new());
    }
    Err(match code {
        Some(code) => format!("{opener} could not open the page (exit {code})"),
        None => format!("{opener} could not open the page"),
    })
}

/// A child process that opens no console window on Windows, where a
/// console program started from a windowed app would show one.
pub(crate) fn new_command(program: impl AsRef<std::ffi::OsStr>) -> Command {
    #[allow(clippy::disallowed_methods)]
    let mut command = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt as _;
        /// `CREATE_NO_WINDOW`, from the Win32 process creation flags.
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(not(windows))]
    let _ = &mut command;
    command
}

/// `command`'s output, as [`Command::output`] gives it, after writing
/// `input` to it, unless it runs longer than `deadline`: then it is stopped
/// and the error is [`io::ErrorKind::TimedOut`]. Its output is read as it
/// prints, so a full pipe never stalls it.
pub(crate) fn output_within(
    command: &mut Command,
    input: &[u8],
    deadline: Duration,
) -> io::Result<Output> {
    let mut child = command
        .stdin(if input.is_empty() {
            Stdio::null()
        } else {
            Stdio::piped()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        let input = input.to_vec();
        // Dropped once written: the end of input.
        thread::spawn(move || stdin.write_all(&input));
    }
    let stdout = drain(child.stdout.take());
    let stderr = drain(child.stderr.take());
    let started = Instant::now();
    let mut pause = Duration::from_millis(1);
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if started.elapsed() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(io::Error::new(io::ErrorKind::TimedOut, "timed out"));
        }
        thread::sleep(pause);
        pause = (pause * 2).min(Duration::from_millis(20));
    };
    Ok(Output {
        status,
        stdout: stdout.join().unwrap_or_default(),
        stderr: stderr.join().unwrap_or_default(),
    })
}

/// Reads all of `pipe` on a thread of its own.
fn drain(pipe: Option<impl Read + Send + 'static>) -> thread::JoinHandle<Vec<u8>> {
    thread::spawn(move || {
        let mut bytes = Vec::new();
        if let Some(mut pipe) = pipe {
            let _ = pipe.read_to_end(&mut bytes);
        }
        bytes
    })
}

/// The mods `[settings] writers` names.
fn writers(settings: &Data) -> Vec<String> {
    match settings.get("writers") {
        Some(Data::List(names)) => names
            .iter()
            .filter_map(Data::as_str)
            .map(str::to_owned)
            .collect(),
        _ => Vec::new(),
    }
}

fn may_act(writers: &[String], from: &str) -> bool {
    writers.iter().any(|writer| writer == from)
}

/// `path` under the home folder, as `~/...`.
fn tilde(path: &str, home: &str) -> String {
    let home = home.trim_end_matches('/');
    if home.is_empty() {
        return path.to_owned();
    }
    match path.strip_prefix(home) {
        Some("" | "/") => "~".to_owned(),
        Some(rest) if rest.starts_with('/') => format!("~{}", rest.trim_end_matches('/')),
        _ => path.to_owned(),
    }
}

/// Now, in ms since the epoch, as herdr-core counts `status_ms`.
fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| {
            i64::try_from(since.as_millis()).unwrap_or(i64::MAX)
        })
}

/// `ms` rounded down to the minute: the clock the views count ages by
/// ("4m"), which changes what is published once a minute, not each tick.
fn to_minute(ms: i64) -> i64 {
    ms - ms.rem_euclid(60_000)
}

/// The panes of this app whose shell finished and has not started again
/// (Enter in the pane starts a new one, under the same key), on any
/// backend.
fn ended_panes(mut finished: Vec<String>, live: &[Value]) -> Vec<String> {
    finished.retain(|pane| {
        !live
            .iter()
            .any(|terminal| terminal.get("pane").and_then(Value::as_str) == Some(pane.as_str()))
    });
    finished.sort();
    finished.dedup();
    finished
}

fn counts(terminals: &[Value]) -> Value {
    let with = |status: &str| {
        terminals
            .iter()
            .filter(|terminal| terminal.get("status").and_then(Value::as_str) == Some(status))
            .count()
    };
    let agents = terminals
        .iter()
        .filter(|terminal| terminal.get("is_agent").and_then(Value::as_bool) == Some(true))
        .count();
    json!({
        "live": terminals.len(),
        "agents": agents,
        "working": with("working"),
        "blocked": with("blocked"),
        "done": with("done"),
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests;
