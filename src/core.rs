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
//!   terminals = { { address, name, agent, cwd, status, turn, ours, pane, is_agent } },
//!   counts = { live, agents, working, blocked, done },
//!   ended = { "p3", ... },   -- this app's panes whose shell exited
//!   peek = { address, text },
//!   app_version,             -- this app's version
//!   machines = { ok, error, list = { { name, target, state, version } } } }
//! ```
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
//!
//! Any mod may send two requests that touch no terminal: `{ refresh =
//! "machines" }` (read `hc machine ls` again at once, rather than at the
//! next look at the server) and `{ open = "releases" }` (open this app's
//! releases page in the browser: the only address it opens), answered as
//! `{ reply = "open", address, ok, error }`. Each runs at most once a tick.

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
/// Where this app's releases are, for Settings > Updates.
const RELEASES: &str = "https://github.com/penso/herdr-moonframe/releases";

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
    /// The machines herdr-core reaches over ssh, read as often as `server`.
    machines: Value,
    /// The tick of the last refresh and of the last page opened: any mod
    /// may ask for either, so each runs at most once a tick.
    refreshed: Option<u64>,
    opened: Option<u64>,
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
            machines: Value::Null,
            refreshed: None,
            opened: None,
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
        if self.ticks.is_multiple_of(SERVER_EVERY) || self.machines.is_null() {
            self.machines = machines(self.hc_json(&["machine", "ls", "--json"]));
        }
        // Finished terminals too: a pane whose shell exited says so.
        let listed = match self.hc_json(&["ls", "--all", "--json"]) {
            Ok(listed) => listed,
            Err(error) => {
                // Settings still shows the app's version and why the
                // machines could not be read.
                return json!({
                    "ok": false,
                    "error": error,
                    "terminals": [],
                    "counts": counts(&[]),
                    "app_version": env!("CARGO_PKG_VERSION"),
                    "machines": self.machines,
                });
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
            "app_version": env!("CARGO_PKG_VERSION"),
            "machines": self.machines,
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
        // What touches no terminal, any mod may ask.
        if message.get("refresh").and_then(Data::as_str) == Some("machines") {
            if once_a_tick(&mut self.refreshed, self.ticks) {
                self.machines = Value::Null;
                self.publish(host);
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
        self.publish(host);
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

/// What `hc machine ls --json` said, as the mods read it: the machines
/// added for good (not ones a command reached once), or why it failed.
fn machines(listed: Result<Value, String>) -> Value {
    let listed = match listed {
        Ok(listed) => listed,
        Err(error) => return json!({ "ok": false, "error": error, "list": [] }),
    };
    let list: Vec<Value> = listed
        .get("machines")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|machine| machine.get("temporary").and_then(Value::as_bool) != Some(true))
        .map(|machine| {
            json!({
                "name": text(machine, "name"),
                "target": text(machine, "target"),
                "state": text(machine, "state"),
                "version": text(machine, "version"),
            })
        })
        .filter(|machine| machine["name"] != "")
        .collect();
    json!({ "ok": true, "error": "", "list": list })
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
    !status.is_empty() || AGENTS.contains(&agent)
}

/// One terminal as the mods read it.
fn entry(terminal: &Value, labels: &Labels, home: &str) -> Value {
    let agent = text(terminal, "agent");
    let status = text(terminal, "status");
    json!({
        "address": text(terminal, "address"),
        "name": text(terminal, "name"),
        "cwd": tilde(&text(terminal, "cwd"), home),
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
            .map(|terminal| entry(terminal, &ours, "/home/u"))
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
    fn machines_are_listed_without_temporary_ones() {
        let listed = json!({ "machines": [
            { "name": "vps", "target": "me@vps", "state": "connected", "version": "0.4.1" },
            { "name": "once", "target": "once", "state": "connected", "temporary": true },
            { "name": "mini", "target": "mini.local", "state": "offline" },
            { "target": "nameless" },
        ] });
        let machines = machines(Ok(listed));
        assert_eq!(machines["ok"], true);
        let names: Vec<&str> = machines["list"]
            .as_array()
            .unwrap()
            .iter()
            .map(|machine| machine["name"].as_str().unwrap())
            .collect();
        assert_eq!(names, ["vps", "mini"]);
        assert_eq!(machines["list"][0]["target"], "me@vps");
        assert_eq!(machines["list"][0]["version"], "0.4.1");
        assert_eq!(machines["list"][1]["version"], "");
    }

    #[test]
    fn machines_that_cannot_be_read_say_why() {
        let machines = machines(Err("hc machine: refused".to_owned()));
        assert_eq!(machines["ok"], false);
        assert_eq!(machines["error"], "hc machine: refused");
        assert_eq!(machines["list"], json!([]));
        assert_eq!(super::machines(Ok(json!({})))["list"], json!([]));
    }

    #[test]
    fn only_the_releases_page_opens() {
        assert_eq!(page_address("releases"), Some(RELEASES));
        assert_eq!(page_address("https://example.com"), None);
        assert!(open_page("file:///etc/passwd").is_err());
    }

    #[test]
    #[cfg(not(windows))]
    fn a_failed_opener_is_reported() {
        assert!(opened("open", true, Some(0)).is_ok());
        let error = opened("xdg-open", false, Some(3)).unwrap_err();
        assert!(error.contains("exit 3"), "{error}");
        assert!(opened("open", false, None).is_err());
    }

    #[test]
    fn requests_any_mod_may_send_run_once_a_tick() {
        let mut last = None;
        assert!(once_a_tick(&mut last, 4));
        assert!(!once_a_tick(&mut last, 4));
        assert!(once_a_tick(&mut last, 5));
    }

    #[test]
    fn replies_carry_the_action_and_its_outcome() {
        let ok = reply("open", "releases", Ok(String::new()));
        assert_eq!(ok.get("reply").and_then(Data::as_str), Some("open"));
        assert_eq!(ok.get("ok"), Some(&Data::Bool(true)));
        let failed = reply("send", "t1", Err("gone".to_owned()));
        assert_eq!(failed.get("ok"), Some(&Data::Bool(false)));
        assert_eq!(failed.get("error").and_then(Data::as_str), Some("gone"));
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
