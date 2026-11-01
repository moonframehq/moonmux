//! `app.core`: what herdr-core runs, read through its command line (`hc`),
//! and what the app asks of it.
//!
//! Each tick lists the live terminals (`hc ls`). A terminal seen for the
//! first time is looked at once (`hc get`) for its labels: the
//! `moonframe-app` label says it is one of this app's panes, and the
//! `moonframe` label (`pane#<id>/term`) which one. So the sidebar can show
//! the agent working in a pane, and an agent started anywhere else (another
//! app, `hc run`, an agent's worker) shows too, to be peeked at.
//!
//! What it publishes:
//!
//! ```text
//! { ok, error, now, core = { name, version },
//!   terminals = { { address, name, agent, program, cwd, status, status_ms,
//!                   turn, parent, ours, pane, is_agent } },
//!   counts = { live, agents, working, blocked, done },
//!   ended = { "p3", ... },   -- this app's panes whose shell exited
//!   peek = { address, text } }
//! ```
//!
//! `program` is the agent's display name (Claude Code). `status_ms` is when
//! the terminal entered its status (ms since the epoch; 0 without one), and
//! `now` the time rounded down to the minute, for the views to count ages
//! by: Luau has no clock. `parent` is the terminal that started it, if any.
//!
//! What it takes: `{ watch = "t7" }` (publish that terminal's screen as
//! `peek` each tick; `""` stops), `{ send = "t7", text }`,
//! `{ keys = "t7", key = "enter" }`, `{ interrupt = "t7" }`,
//! `{ close_panes = "p1,p4" }` (end the shells of those panes of this app).
//! Actions are answered with `{ reply = <action>, address, ok, error }`.
//!
//! These reach any terminal in herdr-core, not only this app's: a peek
//! answers agents running elsewhere. So only the mods named in
//! provider.toml's `[settings] writers` may send them, as Moonframe's git
//! provider does; a mod from a package the user installed is refused.

use {
    moonframe::{Data, Handle, Native},
    serde_json::{Value, json},
    std::{
        collections::HashMap,
        path::{Path, PathBuf},
        process::Command,
    },
};

/// The label herdr-core keeps the owning app's name in.
const APP_LABEL: &str = "moonframe-app";
/// The label holding a terminal's key in its app: `<owner>/<id>`.
const KEY_LABEL: &str = "moonframe";
/// The mod whose instances are the app's panes; `pane#<id>` owns each.
const PANE_MOD: &str = "pane#";
/// Ticks between looks at the server itself (its name and version).
const SERVER_EVERY: u64 = 30;
/// Lines of history above the screen a peek shows.
const PEEK_LINES: &str = "60";

/// Programs herdr-core names that are coding agents, as it reports them,
/// and the name a person knows each by.
const AGENTS: &[(&str, &str)] = &[
    // keep-sorted start
    ("amp", "Amp"),
    ("claude", "Claude Code"),
    ("claude-code", "Claude Code"),
    ("codex", "Codex"),
    ("copilot", "Copilot"),
    ("cursor", "Cursor"),
    ("cursor-agent", "Cursor"),
    ("droid", "Droid"),
    ("gemini", "Gemini"),
    ("grok", "Grok"),
    ("kiro", "Kiro"),
    ("opencode", "OpenCode"),
    ("pi", "Pi"),
    ("qwen", "Qwen Code"),
    // keep-sorted end
];

/// What `hc get` said of a terminal that does not change while it lives.
#[derive(Clone, Debug, Default, PartialEq)]
struct Labels {
    /// One of this app's terminals.
    ours: bool,
    /// The pane instance it is drawn in, for one of ours.
    pane: String,
}

/// When a terminal's status last changed, by herdr-core's clock (`status_ms`
/// in `hc get`; `hc ls` leaves it out), and the status and turn it was read
/// for: a new one is read only when they change.
#[derive(Clone, Debug, Default, PartialEq)]
struct Stamp {
    status: String,
    turn: i64,
    ms: i64,
}

pub(crate) struct Core {
    /// The `hc` found on this machine.
    hc: Option<PathBuf>,
    /// Labels by address, for the terminals live at the last tick.
    labels: HashMap<String, Labels>,
    /// Status times by address, for the terminals live at the last tick.
    stamps: HashMap<String, Stamp>,
    /// The terminal shown in a peek, if any.
    watching: String,
    /// The server's name and version, looked at now and then.
    server: Value,
    ticks: u64,
    /// What was last published, so an unchanged tick redraws nothing.
    last: Value,
    /// The mods that may watch and act (`[settings] writers`).
    writers: Vec<String>,
}

impl Core {
    pub(crate) fn new() -> Self {
        Self {
            hc: find_hc(),
            labels: HashMap::new(),
            stamps: HashMap::new(),
            watching: String::new(),
            server: Value::Null,
            ticks: 0,
            last: Value::Null,
            writers: Vec::new(),
        }
    }

    /// Runs `hc args...` and returns what it printed, or why it failed.
    fn hc(&self, args: &[&str]) -> Result<String, String> {
        let Some(hc) = &self.hc else {
            return Err("herdr-core is not installed: `hc` was not found".to_owned());
        };
        let output = new_command(hc)
            .args(args)
            .output()
            .map_err(|error| format!("hc {}: {error}", args.first().unwrap_or(&"")))?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).into_owned())
        } else {
            let said = String::from_utf8_lossy(&output.stderr).trim().to_owned();
            Err(if said.is_empty() {
                format!("hc {} failed", args.first().unwrap_or(&""))
            } else {
                said
            })
        }
    }

    fn hc_json(&self, args: &[&str]) -> Result<Value, String> {
        let text = self.hc(args)?;
        serde_json::from_str(&text).map_err(|error| format!("hc {}: {error}", args.join(" ")))
    }

    /// The labels of `address`, from the cache or `hc get`.
    fn labels_of(&mut self, address: &str) -> Labels {
        if let Some(labels) = self.labels.get(address) {
            return labels.clone();
        }
        let labels = self
            .hc_json(&["get", address, "--json"])
            .map(|terminal| labels_from(&terminal))
            .unwrap_or_default();
        self.labels.insert(address.to_owned(), labels.clone());
        labels
    }

    /// When `terminal` (from `hc ls`) entered its status, in ms since the
    /// epoch: from the cache while its status and turn stay, else `hc get`.
    /// 0 for a terminal reporting no status.
    fn status_ms(&mut self, terminal: &Value) -> i64 {
        let status = text(terminal, "status");
        if status.is_empty() {
            return 0;
        }
        let address = text(terminal, "address");
        let turn = turn(terminal);
        if let Some(stamp) = self.stamps.get(&address)
            && stamp.status == status
            && stamp.turn == turn
        {
            return stamp.ms;
        }
        // A failed read is not kept: the next tick tries again.
        let Ok(got) = self.hc_json(&["get", &address, "--json"]) else {
            return 0;
        };
        let ms = got.get("status_ms").and_then(Value::as_i64).unwrap_or(0);
        self.stamps.insert(address, Stamp { status, turn, ms });
        ms
    }

    fn snapshot(&mut self) -> Value {
        if (self.ticks.is_multiple_of(SERVER_EVERY) || self.server.is_null())
            && let Ok(status) = self.hc_json(&["server", "status", "--json"])
        {
            self.server = json!({
                "name": status.pointer("/core/name").and_then(Value::as_str).unwrap_or(""),
                "version": status.get("version").and_then(Value::as_str).unwrap_or(""),
            });
        }
        // Finished terminals too: a pane whose shell exited says so.
        let listed = match self.hc_json(&["ls", "--all", "--json"]) {
            Ok(listed) => listed,
            Err(error) => {
                return json!({ "ok": false, "error": error, "terminals": [], "counts": counts(&[]) });
            }
        };
        let home = std::env::var("HOME").unwrap_or_default();
        let raw = listed
            .get("terminals")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        // Forget the labels of terminals that are gone: an address can be
        // given again later.
        let live: Vec<String> = raw
            .iter()
            .filter_map(|terminal| terminal.get("address").and_then(Value::as_str))
            .map(str::to_owned)
            .collect();
        self.labels.retain(|address, _| live.contains(address));
        self.stamps.retain(|address, _| live.contains(address));
        let mut terminals = Vec::new();
        let mut finished = Vec::new();
        for terminal in &raw {
            let address = text(terminal, "address");
            let labels = self.labels_of(&address);
            if text(terminal, "state") == "live" {
                let since = self.status_ms(terminal);
                terminals.push(entry(terminal, &labels, since, &home));
            } else if labels.ours && !labels.pane.is_empty() {
                finished.push(labels.pane);
            }
        }
        let ended = ended_panes(finished, &terminals);
        let mut snapshot = json!({
            "ok": true,
            "error": "",
            "now": to_minute(now_ms()),
            "core": self.server,
            "counts": counts(&terminals),
            "terminals": terminals,
            "ended": ended,
        });
        if !self.watching.is_empty() {
            let watching = self.watching.clone();
            let peek = match self.hc_json(&["read", &watching, "--lines", PEEK_LINES, "--json"]) {
                Ok(read) => {
                    json!({ "address": watching, "text": text(&read, "text"), "error": "" })
                }
                Err(error) => json!({ "address": watching, "text": "", "error": error }),
            };
            snapshot["peek"] = peek;
        }
        snapshot
    }

    fn publish(&mut self, host: &Handle) {
        let snapshot = self.snapshot();
        if snapshot != self.last {
            host.publish_json(&snapshot);
            self.last = snapshot;
        }
    }

    /// An action a mod asked for: what `hc` was told, and whether it took.
    fn act(&mut self, message: &Data) -> Option<(&'static str, String, Result<String, String>)> {
        let field = |key: &str| message.get(key).and_then(Data::as_str).map(str::to_owned);
        if let Some(address) = field("send") {
            let text = field("text").unwrap_or_default();
            let result = self.hc(&["send", &address, &text]);
            return Some(("send", address, result));
        }
        if let Some(address) = field("keys") {
            let key = field("key").unwrap_or_else(|| "enter".to_owned());
            let result = self.hc(&["keys", &address, &key]);
            return Some(("keys", address, result));
        }
        if let Some(address) = field("interrupt") {
            let result = self.hc(&["interrupt", &address]);
            return Some(("interrupt", address, result));
        }
        if let Some(panes) = field("close_panes") {
            let result = self.close_panes(&panes);
            return Some(("close_panes", panes, result));
        }
        None
    }

    /// Ends the shells of this app's panes `panes` (`p1,p4`): closing a
    /// pane ends what runs in it, as closing the window does not.
    fn close_panes(&mut self, panes: &str) -> Result<String, String> {
        let panes: Vec<&str> = panes.split(',').filter(|pane| !pane.is_empty()).collect();
        let listed = self.hc_json(&[
            "ls",
            "--json",
            "--label",
            &format!("{APP_LABEL}={}", crate::NAME),
        ])?;
        let addresses: Vec<String> = listed
            .get("terminals")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .map(|terminal| text(terminal, "address"))
            .collect();
        let mut problems = Vec::new();
        for address in addresses {
            let labels = self.labels_of(&address);
            if labels.ours
                && panes.contains(&labels.pane.as_str())
                && let Err(error) = self.hc(&["kill", &address])
            {
                problems.push(format!("{address}: {error}"));
            }
        }
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
        self.publish(host);
        Ok(())
    }

    fn tick(&mut self, host: &Handle) {
        self.ticks += 1;
        self.publish(host);
    }

    fn receive(&mut self, host: &Handle, from: &str, message: Data) {
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
            host.send(
                from,
                Data::map([
                    ("reply", action.into()),
                    ("address", address.into()),
                    ("ok", result.is_ok().into()),
                    ("error", result.err().unwrap_or_default().into()),
                ]),
            );
        }
        self.publish(host);
    }
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

/// A string field of `value`, or `""`.
fn text(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned()
}

/// What `hc get`'s labels say about a terminal.
fn labels_from(terminal: &Value) -> Labels {
    let labels = terminal.get("labels");
    let label = |key: &str| {
        labels
            .and_then(|labels| labels.get(key))
            .and_then(Value::as_str)
            .unwrap_or("")
    };
    let ours = label(APP_LABEL) == crate::NAME;
    Labels {
        ours,
        pane: if ours {
            pane_of(label(KEY_LABEL))
        } else {
            String::new()
        },
    }
}

/// The pane id in a terminal key: `pane#p12/term` is `p12`.
fn pane_of(key: &str) -> String {
    key.strip_prefix(PANE_MOD)
        .and_then(|rest| rest.split_once('/'))
        .map(|(pane, _)| pane.to_owned())
        .unwrap_or_default()
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

fn is_agent(agent: &str, status: &str) -> bool {
    !status.is_empty() || AGENTS.iter().any(|(known, _)| *known == agent)
}

/// The name a person knows `agent` by (`claude-code` is Claude Code), or
/// the program's own name.
fn program_name(agent: &str) -> &str {
    AGENTS
        .iter()
        .find(|(known, _)| *known == agent)
        .map_or(agent, |(_, name)| name)
}

fn turn(terminal: &Value) -> i64 {
    terminal.get("turn").and_then(Value::as_i64).unwrap_or(0)
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

/// One terminal as the mods read it; `status_ms` is when it entered its
/// status.
fn entry(terminal: &Value, labels: &Labels, status_ms: i64, home: &str) -> Value {
    let agent = text(terminal, "agent");
    let status = text(terminal, "status");
    json!({
        "address": text(terminal, "address"),
        "name": text(terminal, "name"),
        "cwd": tilde(&text(terminal, "cwd"), home),
        "program": program_name(&agent),
        "agent": agent,
        "is_agent": is_agent(&agent, &status),
        "status": status,
        "status_ms": status_ms,
        "parent": text(terminal, "parent"),
        "turn": turn(terminal),
        "ours": labels.ours,
        "pane": labels.pane,
    })
}

/// The panes of this app whose shell finished and has not started again
/// (Enter in the pane starts a new one, under the same key).
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

/// `hc`: `$HC_BIN`, else on `PATH`, else where its installer puts it (an
/// app opened from the Dock has a short `PATH`).
fn find_hc() -> Option<PathBuf> {
    if let Some(bin) = std::env::var_os("HC_BIN").map(PathBuf::from)
        && bin.is_file()
    {
        return Some(bin);
    }
    let on_path = std::env::var_os("PATH")
        .into_iter()
        .flat_map(|path| std::env::split_paths(&path).collect::<Vec<_>>())
        .map(|dir| dir.join("hc"))
        .find(|candidate| candidate.is_file());
    on_path.or_else(|| {
        let home = std::env::var_os("HOME").map(PathBuf::from)?;
        [".local/bin/hc", ".cargo/bin/hc"]
            .iter()
            .map(|relative| home.join(relative))
            .chain([Path::new("/opt/homebrew/bin/hc").to_owned()])
            .find(|candidate| candidate.is_file())
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn pane_ids_come_from_the_key_label() {
        assert_eq!(pane_of("pane#p12/term"), "p12");
        assert_eq!(pane_of("pane#p12"), "");
        assert_eq!(pane_of("session/s7114"), "");
        assert_eq!(pane_of(""), "");
    }

    #[test]
    fn only_this_apps_terminals_are_ours() {
        let ours = json!({ "labels": { "moonframe-app": "herdr-moonframe", "moonframe": "pane#p3/term" } });
        assert_eq!(
            labels_from(&ours),
            Labels {
                ours: true,
                pane: "p3".into()
            }
        );
        let theirs = json!({ "labels": { "moonframe-app": "moonframe-sessions", "moonframe": "pane#p3/term" } });
        assert_eq!(labels_from(&theirs), Labels::default());
        assert_eq!(labels_from(&json!({})), Labels::default());
    }

    #[test]
    fn folders_under_home_get_a_tilde() {
        assert_eq!(tilde("/Users/me", "/Users/me"), "~");
        assert_eq!(tilde("/Users/me/", "/Users/me"), "~");
        assert_eq!(tilde("/Users/me/code/x", "/Users/me/"), "~/code/x");
        assert_eq!(tilde("/Users/meow", "/Users/me"), "/Users/meow");
        assert_eq!(tilde("/tmp", ""), "/tmp");
    }

    #[test]
    fn agents_are_known_programs_or_ones_reporting_status() {
        assert!(is_agent("claude-code", ""));
        assert!(is_agent("my-bot", "working"));
        assert!(!is_agent("zsh", ""));
    }

    #[test]
    fn entries_and_counts() {
        let listed = [
            json!({ "address": "t1", "agent": "zsh", "cwd": "/home/u/x", "state": "live", "turn": 0 }),
            json!({ "address": "t2", "agent": "claude-code", "cwd": "/home/u", "status": "working", "turn": 3 }),
            json!({ "address": "t3", "agent": "codex", "cwd": "/srv", "status": "blocked", "turn": 1 }),
        ];
        let ours = Labels {
            ours: true,
            pane: "p1".into(),
        };
        let entries: Vec<Value> = listed
            .iter()
            .map(|terminal| entry(terminal, &ours, 0, "/home/u"))
            .collect();
        assert_eq!(entries[0]["cwd"], "~/x");
        assert_eq!(entries[0]["is_agent"], false);
        assert_eq!(entries[1]["status"], "working");
        assert_eq!(entries[1]["pane"], "p1");
        assert_eq!(entries[2]["cwd"], "/srv");
        let counts = counts(&entries);
        assert_eq!(counts["live"], 3);
        assert_eq!(counts["agents"], 2);
        assert_eq!(counts["working"], 1);
        assert_eq!(counts["blocked"], 1);
        assert_eq!(counts["done"], 0);
    }

    #[test]
    fn entries_say_the_program_its_parent_and_since_when() {
        let helper = json!({ "address": "t4", "agent": "claude", "status": "working", "parent": "t2", "turn": 1 });
        let entry = entry(&helper, &Labels::default(), 1_793_567_863_379, "/home/u");
        assert_eq!(entry["program"], "Claude Code");
        assert_eq!(entry["parent"], "t2");
        assert_eq!(entry["status_ms"], 1_793_567_863_379_i64);
        let alone = json!({ "address": "t5", "agent": "my-bot" });
        let alone = super::entry(&alone, &Labels::default(), 0, "/home/u");
        assert_eq!(alone["program"], "my-bot");
        assert_eq!(alone["parent"], "");
    }

    #[test]
    fn programs_are_known_by_their_names() {
        assert_eq!(program_name("claude-code"), "Claude Code");
        assert_eq!(program_name("cursor-agent"), "Cursor");
        assert_eq!(program_name("zsh"), "zsh");
        assert_eq!(program_name(""), "");
    }

    #[test]
    fn the_clock_is_published_by_the_minute() {
        assert_eq!(to_minute(1_793_567_863_379), 1_793_567_820_000);
        assert_eq!(to_minute(1_793_567_820_000), 1_793_567_820_000);
        assert_eq!(to_minute(59_999), 0);
        assert!(now_ms() > 1_700_000_000_000);
    }

    #[test]
    fn status_times_are_read_again_only_when_the_status_changes() {
        let mut core = Core {
            hc: None,
            ..Core::new()
        };
        // No status: nothing to read.
        assert_eq!(
            core.status_ms(&json!({ "address": "t1", "agent": "zsh" })),
            0
        );
        assert!(core.stamps.is_empty());
        let working = json!({ "address": "t2", "status": "working", "turn": 3 });
        core.stamps.insert(
            "t2".into(),
            Stamp {
                status: "working".into(),
                turn: 3,
                ms: 42,
            },
        );
        assert_eq!(core.status_ms(&working), 42);
        // A new status is looked up. Here `hc` is missing, so the read
        // fails: it says 0 and keeps the old stamp, to try again next tick.
        let done = json!({ "address": "t2", "status": "done", "turn": 3 });
        assert_eq!(core.status_ms(&done), 0);
        assert_eq!(core.stamps["t2"].status, "working");
    }

    #[test]
    fn only_the_named_writers_may_act() {
        let settings = Data::map([("writers", vec!["peek", "workspace"].into())]);
        let writers = writers(&settings);
        assert!(may_act(&writers, "peek"));
        assert!(may_act(&writers, "workspace"));
        assert!(!may_act(&writers, "sidebar"));
        assert!(!may_act(&writers, "someones-package"));
    }

    #[test]
    fn without_writers_no_mod_may_act() {
        let none = writers(&Data::Nil);
        assert!(none.is_empty());
        assert!(!may_act(&none, "peek"));
    }

    #[test]
    fn a_pane_has_ended_until_its_shell_starts_again() {
        let live = [json!({ "pane": "p2" })];
        let finished = vec!["p3".to_owned(), "p2".to_owned(), "p3".to_owned()];
        assert_eq!(ended_panes(finished, &live), vec!["p3".to_owned()]);
    }

    #[test]
    fn a_missing_hc_says_so() {
        let core = Core {
            hc: None,
            ..Core::new()
        };
        let error = core.hc(&["ls"]).unwrap_err();
        assert!(error.contains("not installed"), "{error}");
    }
}
