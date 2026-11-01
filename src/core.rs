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
//! { ok, error, core = { name, version },
//!   terminals = { { address, name, agent, program, program_key, cwd, status,
//!                   turn, ours, pane, is_agent } },
//!   counts = { live, agents, working, blocked, done },
//!   ended = { "p3", ... },   -- this app's panes whose shell exited
//!   programs = { { key, name, installed } },   -- the agents "+" can start
//!   peek = { address, text } }
//! ```
//!
//! A terminal's `program` is what to call it (`Claude Code`, `zsh`), and
//! `program_key` the key its icon is drawn by (`claude`, `shell`, or `""`
//! for one the app does not know): see programs.rs.
//!
//! What it takes: `{ watch = "t7" }` (publish that terminal's screen as
//! `peek` each tick; `""` stops), `{ send = "t7", text }`,
//! `{ keys = "t7", key = "enter" }`, `{ interrupt = "t7" }`,
//! `{ close_panes = "p1,p4" }` (end the shells of those panes of this app),
//! `{ start = "p3", program = "claude" }` (type a startable agent's command
//! into that pane's new shell, once: the terminal is labelled first, and a
//! labelled one is never typed into again, across reloads and launches).
//! Actions are answered with `{ reply = <action>, address, ok, error }`.
//!
//! These reach any terminal in herdr-core, not only this app's: a peek
//! answers agents running elsewhere. So only the mods named in
//! provider.toml's `[settings] writers` may send them, as Moonframe's git
//! provider does; a mod from a package the user installed is refused.

use {
    moonframe::{Data, Handle, Native},
    serde_json::{Value, json},
    std::{collections::HashMap, path::PathBuf, process::Command},
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
/// The label marking a pane's terminal as given its agent: `start` types
/// into a terminal only while it has none.
const START_LABEL: &str = "herdr-moonframe-start";
/// Ticks between looks for installed agents.
const PROGRAMS_EVERY: u64 = 10;

/// Programs herdr-core names that are coding agents, as it reports them.
const AGENTS: &[&str] = &[
    // keep-sorted start
    "amp",
    "claude",
    "claude-code",
    "codex",
    "copilot",
    "cursor",
    "cursor-agent",
    "droid",
    "gemini",
    "grok",
    "kiro",
    "opencode",
    "pi",
    "qwen",
    // keep-sorted end
];

/// What `hc get` said of a terminal that does not change while it lives.
#[derive(Clone, Debug, Default, PartialEq)]
struct Labels {
    /// One of this app's terminals.
    ours: bool,
    /// The pane instance it is drawn in, for one of ours.
    pane: String,
    /// The agent `start` typed into it, for one of ours.
    started: String,
}

pub(crate) struct Core {
    /// The `hc` found on this machine.
    hc: Option<PathBuf>,
    /// Labels by address, for the terminals live at the last tick.
    labels: HashMap<String, Labels>,
    /// The terminal shown in a peek, if any.
    watching: String,
    /// The server's name and version, looked at now and then.
    server: Value,
    /// The startable agents and whether each is installed, looked at now
    /// and then.
    programs: Value,
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
            watching: String::new(),
            server: Value::Null,
            programs: Value::Null,
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

    fn snapshot(&mut self) -> Value {
        if (self.ticks.is_multiple_of(SERVER_EVERY) || self.server.is_null())
            && let Ok(status) = self.hc_json(&["server", "status", "--json"])
        {
            self.server = json!({
                "name": status.pointer("/core/name").and_then(Value::as_str).unwrap_or(""),
                "version": status.get("version").and_then(Value::as_str).unwrap_or(""),
            });
        }
        if self.ticks.is_multiple_of(PROGRAMS_EVERY) || self.programs.is_null() {
            self.programs = crate::programs::installed();
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
        let mut terminals = Vec::new();
        let mut finished = Vec::new();
        for terminal in &raw {
            let address = text(terminal, "address");
            let labels = self.labels_of(&address);
            if text(terminal, "state") == "live" {
                terminals.push(entry(terminal, &labels, &home));
            } else if labels.ours && !labels.pane.is_empty() {
                finished.push(labels.pane);
            }
        }
        let ended = ended_panes(finished, &terminals);
        let mut snapshot = json!({
            "ok": true,
            "error": "",
            "core": self.server,
            "counts": counts(&terminals),
            "terminals": terminals,
            "ended": ended,
            "programs": self.programs,
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
        if let Some(pane) = field("start") {
            let result = self.start(&pane, &field("program").unwrap_or_default());
            return Some(("start", pane, result));
        }
        None
    }

    /// The live terminals of this app (the label says so; `labels_of`
    /// tells which pane each is).
    fn our_terminals(&self) -> Result<Vec<String>, String> {
        let listed = self.hc_json(&[
            "ls",
            "--json",
            "--label",
            &format!("{APP_LABEL}={}", crate::NAME),
        ])?;
        Ok(listed
            .get("terminals")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .map(|terminal| text(terminal, "address"))
            .collect())
    }

    /// Types the startable agent `key`'s command into the new shell of this
    /// app's pane `pane`, once: the terminal is labelled before it is typed
    /// into, and a labelled terminal is left alone, so neither a message
    /// sent twice nor a reload starts a second agent.
    fn start(&mut self, pane: &str, key: &str) -> Result<String, String> {
        let program = crate::programs::startable(key)
            .ok_or_else(|| format!("`{key}` is not an agent this app starts"))?;
        let found = crate::programs::locate(program.run)
            .ok_or_else(|| format!("{} is not installed", program.name))?;
        let mut address = None;
        for candidate in self.our_terminals()? {
            if self.labels_of(&candidate).pane == pane {
                address = Some(candidate);
                break;
            }
        }
        let address = address.ok_or_else(|| format!("pane {pane} has no live terminal"))?;
        if !self.labels_of(&address).started.is_empty() {
            return Ok("already started".to_owned());
        }
        self.hc(&["label", &address, &format!("{START_LABEL}={key}")])?;
        if let Some(labels) = self.labels.get_mut(&address) {
            labels.started = key.to_owned();
        }
        let path = std::env::var_os("PATH");
        let command = crate::programs::command_line(program, &found, path.as_deref());
        self.hc(&["send", &address, &command])
    }

    /// Ends the shells of this app's panes `panes` (`p1,p4`): closing a
    /// pane ends what runs in it, as closing the window does not.
    fn close_panes(&mut self, panes: &str) -> Result<String, String> {
        let panes: Vec<&str> = panes.split(',').filter(|pane| !pane.is_empty()).collect();
        let addresses = self.our_terminals()?;
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
    if !ours {
        return Labels::default();
    }
    Labels {
        ours,
        pane: pane_of(label(KEY_LABEL)),
        started: label(START_LABEL).to_owned(),
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
    !status.is_empty() || AGENTS.contains(&agent)
}

/// One terminal as the mods read it.
fn entry(terminal: &Value, labels: &Labels, home: &str) -> Value {
    let agent = text(terminal, "agent");
    let status = text(terminal, "status");
    let (key, program) = crate::programs::program_of(&agent);
    json!({
        "address": text(terminal, "address"),
        "name": text(terminal, "name"),
        "cwd": tilde(&text(terminal, "cwd"), home),
        "program": program,
        "program_key": key,
        "agent": agent,
        "is_agent": is_agent(&agent, &status),
        "status": status,
        "turn": terminal.get("turn").and_then(Value::as_i64).unwrap_or(0),
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
    crate::programs::locate("hc")
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
                pane: "p3".into(),
                started: String::new(),
            }
        );
        let started = json!({ "labels": {
            "moonframe-app": "herdr-moonframe",
            "moonframe": "pane#p4/term",
            "herdr-moonframe-start": "claude",
        } });
        assert_eq!(labels_from(&started).started, "claude");
        // Another app's label never counts as started here.
        let theirs_started = json!({ "labels": { "herdr-moonframe-start": "claude" } });
        assert_eq!(labels_from(&theirs_started), Labels::default());
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
            started: String::new(),
        };
        let entries: Vec<Value> = listed
            .iter()
            .map(|terminal| entry(terminal, &ours, "/home/u"))
            .collect();
        assert_eq!(entries[0]["cwd"], "~/x");
        assert_eq!(entries[0]["is_agent"], false);
        assert_eq!(entries[0]["program_key"], "shell");
        assert_eq!(entries[0]["program"], "zsh");
        assert_eq!(entries[1]["program_key"], "claude");
        assert_eq!(entries[1]["program"], "Claude Code");
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
    fn start_refuses_what_is_not_a_startable_agent() {
        let mut core = Core {
            hc: None,
            ..Core::new()
        };
        let error = core.start("p1", "rm -rf ~").unwrap_err();
        assert!(error.contains("not an agent"), "{error}");
        let message = Data::map([("start", "p1".into()), ("program", "amp".into())]);
        let (action, pane, result) = core.act(&message).unwrap();
        assert_eq!((action, pane.as_str()), ("start", "p1"));
        assert!(result.is_err());
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
