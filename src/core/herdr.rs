//! The herdr-core backend: what herdr-core runs, read through its command
//! line (`hc`).
//!
//! Each tick lists the terminals (`hc ls --all`). A terminal seen for the
//! first time is looked at once (`hc get`) for its labels: the
//! `moonframe-app` label says it is one of this app's panes, and the
//! `moonframe` label (`pane#<id>/term`) which one. A finished terminal of
//! one of ours says its pane's shell exited. An address is herdr-core's
//! own (`t7`).
//!
//! `hc ls`'s `cwd` is the folder a terminal started in. For this app's own
//! panes, whose folder the Files and Changes panels follow, each tick also
//! asks `hc get` where its foreground program is now (`foreground.cwd`).

use {
    super::{Backend, Listing, new_command, tilde},
    serde_json::{Value, json},
    std::{collections::HashMap, path::PathBuf},
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
const START_LABEL: &str = "moonmux-start";

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

/// When a terminal's status last changed, by herdr-core's clock (`status_ms`
/// in `hc get`; `hc ls` leaves it out), and the status and turn it was read
/// for: a new one is read only when they change.
#[derive(Clone, Debug, Default, PartialEq)]
struct Stamp {
    status: String,
    turn: i64,
    ms: i64,
}

pub(crate) struct HerdrCore {
    /// The `hc` found on this machine.
    hc: Option<PathBuf>,
    /// Labels by address, for the terminals live at the last tick.
    labels: HashMap<String, Labels>,
    /// Status times by address, for the terminals live at the last tick.
    stamps: HashMap<String, Stamp>,
    /// The server's name and version, looked at now and then.
    server: Value,
}

impl HerdrCore {
    pub(crate) fn new() -> Self {
        Self {
            hc: find_hc(),
            labels: HashMap::new(),
            stamps: HashMap::new(),
            server: Value::Null,
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

    /// Where the program in front in `address` is now, from `hc get`; `None`
    /// when herdr-core cannot say.
    fn live_cwd(&self, address: &str) -> Option<String> {
        let got = self.hc_json(&["get", address, "--json"]).ok()?;
        live_cwd_of(&got).map(str::to_owned)
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
}

impl Backend for HerdrCore {
    fn id(&self) -> &'static str {
        "herdr-core"
    }

    fn installed(&self) -> bool {
        self.hc.is_some()
    }

    fn find(&mut self) {
        self.hc = find_hc();
    }

    fn info(&self) -> Value {
        if self.server.is_null() {
            json!({ "name": "", "version": "" })
        } else {
            self.server.clone()
        }
    }

    fn list(&mut self, ticks: u64, home: &str) -> Result<Listing, String> {
        if (ticks.is_multiple_of(SERVER_EVERY) || self.server.is_null())
            && let Ok(status) = self.hc_json(&["server", "status", "--json"])
        {
            self.server = json!({
                "name": status.pointer("/core/name").and_then(Value::as_str).unwrap_or(""),
                "version": status.get("version").and_then(Value::as_str).unwrap_or(""),
            });
        }
        // Finished terminals too: a pane whose shell exited says so.
        let listed = self.hc_json(&["ls", "--all", "--json"])?;
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
        let mut listing = Listing::default();
        for terminal in &raw {
            let address = text(terminal, "address");
            let labels = self.labels_of(&address);
            if text(terminal, "state") == "live" {
                let since = self.status_ms(terminal);
                let mut terminal = terminal.clone();
                if labels.ours
                    && let Some(cwd) = self.live_cwd(&address)
                {
                    terminal["cwd"] = Value::String(cwd);
                }
                listing
                    .terminals
                    .push(entry(&terminal, &labels, since, home));
            } else if labels.ours && !labels.pane.is_empty() {
                listing.finished.push(labels.pane);
            }
        }
        Ok(listing)
    }

    fn owns(&self, address: &str) -> bool {
        // tmux's addresses are `server:%id`; herdr-core's never hold a `%`.
        !address.is_empty() && !address.contains('%')
    }

    fn read(&mut self, address: &str) -> Result<String, String> {
        self.hc_json(&["read", address, "--lines", PEEK_LINES, "--json"])
            .map(|read| text(&read, "text"))
    }

    fn send(&mut self, address: &str, text: &str) -> Result<String, String> {
        self.hc(&["send", address, text])
    }

    fn keys(&mut self, address: &str, key: &str) -> Result<String, String> {
        self.hc(&["keys", address, key])
    }

    fn interrupt(&mut self, address: &str) -> Result<String, String> {
        self.hc(&["interrupt", address])
    }

    fn close_panes(&mut self, panes: &[&str]) -> Result<String, String> {
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

    fn start(&mut self, pane: &str, key: &str, command: &str) -> Result<Option<String>, String> {
        let mut address = None;
        for candidate in self.our_terminals()? {
            if self.labels_of(&candidate).pane == pane {
                address = Some(candidate);
                break;
            }
        }
        let Some(address) = address else {
            return Ok(None);
        };
        if !self.labels_of(&address).started.is_empty() {
            return Ok(Some("already started".to_owned()));
        }
        self.hc(&["label", &address, &format!("{START_LABEL}={key}")])?;
        if let Some(labels) = self.labels.get_mut(&address) {
            labels.started = key.to_owned();
        }
        self.hc(&["send", &address, command]).map(Some)
    }

    fn machines(&mut self) -> Option<Value> {
        Some(machines(self.hc_json(&["machine", "ls", "--json"])))
    }
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

fn is_agent(agent: &str, status: &str) -> bool {
    !status.is_empty() || AGENTS.contains(&agent)
}

fn turn(terminal: &Value) -> i64 {
    terminal.get("turn").and_then(Value::as_i64).unwrap_or(0)
}

/// The folder `hc get` says the foreground program of a terminal is in.
fn live_cwd_of(got: &Value) -> Option<&str> {
    got.pointer("/foreground/cwd")
        .and_then(Value::as_str)
        .filter(|cwd| !cwd.is_empty())
}

/// One terminal as the mods read it; `status_ms` is when it entered its
/// status.
fn entry(terminal: &Value, labels: &Labels, status_ms: i64, home: &str) -> Value {
    let agent = text(terminal, "agent");
    let status = text(terminal, "status");
    let (key, program) = crate::programs::program_of(&agent);
    json!({
        "address": text(terminal, "address"),
        "target": text(terminal, "address"),
        "backend": "herdr-core",
        "name": text(terminal, "name"),
        "cwd": tilde(&text(terminal, "cwd"), home),
        "program": program,
        "program_key": key,
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

/// `hc`: `$HC_BIN`, else on `PATH`, else where its installer puts it (an
/// app opened from the Dock has a short `PATH`).
pub(crate) fn find_hc() -> Option<PathBuf> {
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

    fn missing() -> HerdrCore {
        HerdrCore {
            hc: None,
            ..HerdrCore::new()
        }
    }

    #[test]
    fn pane_ids_come_from_the_key_label() {
        assert_eq!(pane_of("pane#p12/term"), "p12");
        assert_eq!(pane_of("pane#p12"), "");
        assert_eq!(pane_of("session/s7114"), "");
        assert_eq!(pane_of(""), "");
    }

    #[test]
    fn only_this_apps_terminals_are_ours() {
        let ours = json!({ "labels": { "moonframe-app": "moonmux", "moonframe": "pane#p3/term" } });
        assert_eq!(
            labels_from(&ours),
            Labels {
                ours: true,
                pane: "p3".into(),
                started: String::new(),
            }
        );
        let started = json!({ "labels": {
            "moonframe-app": "moonmux",
            "moonframe": "pane#p4/term",
            "moonmux-start": "claude",
        } });
        assert_eq!(labels_from(&started).started, "claude");
        // Another app's label never counts as started here.
        let theirs_started = json!({ "labels": { "moonmux-start": "claude" } });
        assert_eq!(labels_from(&theirs_started), Labels::default());
        let theirs = json!({ "labels": { "moonframe-app": "moonframe-sessions", "moonframe": "pane#p3/term" } });
        assert_eq!(labels_from(&theirs), Labels::default());
        assert_eq!(labels_from(&json!({})), Labels::default());
    }

    #[test]
    fn the_live_folder_is_the_foreground_programs() {
        let got = json!({
            "cwd": "/home/u/",
            "foreground": { "cwd": "/home/u/code/app", "name": "zsh" },
        });
        assert_eq!(live_cwd_of(&got), Some("/home/u/code/app"));
        assert_eq!(live_cwd_of(&json!({ "cwd": "/home/u/" })), None);
        assert_eq!(live_cwd_of(&json!({ "foreground": { "cwd": "" } })), None);
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
            .map(|terminal| entry(terminal, &ours, 0, "/home/u"))
            .collect();
        assert_eq!(entries[0]["cwd"], "~/x");
        assert_eq!(entries[0]["is_agent"], false);
        assert_eq!(entries[0]["program_key"], "shell");
        assert_eq!(entries[0]["program"], "zsh");
        assert_eq!(entries[0]["backend"], "herdr-core");
        assert_eq!(entries[0]["target"], "t1");
        assert_eq!(entries[1]["program_key"], "claude");
        assert_eq!(entries[1]["program"], "Claude Code");
        assert_eq!(entries[1]["status"], "working");
        assert_eq!(entries[1]["pane"], "p1");
        assert_eq!(entries[2]["cwd"], "/srv");
        let counts = super::super::counts(&entries);
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
    fn status_times_are_read_again_only_when_the_status_changes() {
        let mut core = missing();
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
    fn its_addresses_are_the_ones_without_a_percent() {
        let core = missing();
        assert!(core.owns("t7"));
        assert!(!core.owns("default:%3"));
        assert!(!core.owns(""));
    }

    #[test]
    fn a_missing_hc_says_so() {
        let mut core = missing();
        assert!(!core.installed());
        let error = core.hc(&["ls"]).unwrap_err();
        assert!(error.contains("not installed"), "{error}");
        assert!(core.list(0, "/home/u").is_err());
        assert!(core.start("p1", "claude", "claude").is_err());
    }
}
