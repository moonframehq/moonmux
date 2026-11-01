//! The tmux backend: what tmux runs for this user.
//!
//! Moonframe keeps each of this app's tmux terminals as a session on the
//! app's own server (`tmux -L herdr-moonframe`), named after the terminal's
//! key: the pane `p3`'s is `pane#p3/term`, so its session is
//! `pane_p3+term` (with a `~hash` after it on newer Moonframes). Each tick
//! lists the panes of that server and of every other server this user runs
//! (the default one, and any `tmux -L name`), finds the coding agents in
//! them (server.rs, agents.rs), and reads what each agent is doing off its
//! screen. An address is `server:%id` (`default:%12`).
//!
//! A pane of ours seen running in this run whose session is gone has
//! ended. `start` marks a session with the user option
//! `@herdr-moonframe-start` before typing into it, and never types into a
//! marked one.

mod agents;
mod server;

use {
    super::{Backend, Listing, now_ms, tilde},
    agents::{Processes, Track},
    serde_json::{Value, json},
    server::{Failure, Pane, Program},
    std::{
        collections::{HashMap, HashSet},
        path::PathBuf,
    },
};

/// The mod whose instances are the app's panes, and the id of each one's
/// terminal: `pane#p3/term` keeps its shell in session `pane_p3+term`.
const PANE_SESSION_PREFIX: &str = "pane_";
const PANE_SESSION_TERMINAL: &str = "+term";
/// The session option marking a pane's terminal as given its agent.
const START_OPTION: &str = "@herdr-moonframe-start";
/// Ticks between looks for servers that started or stopped.
const SERVERS_EVERY: u64 = 10;
/// Lines of history above the screen a peek shows.
const PEEK_LINES: &str = "-60";
/// Seconds since a pane last printed for it to count as printing still.
const RECENT: u64 = 2;

pub(crate) struct Tmux {
    tmux: Option<Program>,
    version: String,
    /// The app's own server: `tmux -L <this>`.
    socket: String,
    /// Where this user's tmux sockets are.
    socket_dir: Option<PathBuf>,
    /// The other servers this user runs, looked for now and then.
    others: Vec<String>,
    /// What each agent was doing at the last tick, by address.
    tracks: HashMap<String, Track>,
    /// This app's panes seen running in this run: one gone since ended.
    seen_panes: HashSet<String>,
}

impl Tmux {
    pub(crate) fn new() -> Self {
        let tmux = Program::find();
        Self {
            version: tmux.as_ref().map(Program::version).unwrap_or_default(),
            tmux,
            socket: crate::NAME.to_owned(),
            socket_dir: server::socket_dir(),
            others: Vec::new(),
            tracks: HashMap::new(),
            seen_panes: HashSet::new(),
        }
    }

    fn program(&self) -> Result<&Program, String> {
        self.tmux
            .as_ref()
            .ok_or_else(|| "tmux is not installed: `tmux` was not found".to_owned())
    }

    /// Whether the mods may name `server`: this app's, or one found running.
    fn known_server(&self, server: &str) -> bool {
        server == self.socket || self.others.iter().any(|other| other == server)
    }

    /// Runs each of `args` against the pane at `address`, after
    /// `-t <pane>`.
    fn on_pane(&self, address: &str, args: &[&[&str]]) -> Result<String, String> {
        let tmux = self.program()?;
        let Some((server, pane)) = server::split_address(address) else {
            return Err(format!("`{address}` is not a tmux pane"));
        };
        if !self.known_server(server) {
            return Err(format!("no tmux server `{server}` runs here"));
        }
        let mut printed = String::new();
        for each in args {
            let Some((verb, rest)) = each.split_first() else {
                continue;
            };
            let mut command = vec![*verb, "-t", pane];
            command.extend_from_slice(rest);
            printed = tmux
                .run(server, &command)
                .map_err(|failure| failure_text(&failure))?;
        }
        Ok(printed)
    }

    /// The live pane of this app's pane `pane`, if it has one.
    fn our_pane(&self, pane: &str) -> Result<Option<Pane>, String> {
        let tmux = self.program()?;
        Ok(tmux
            .panes(&self.socket)?
            .into_iter()
            .find(|each| !each.dead && pane_of(&each.session) == pane))
    }
}

impl Backend for Tmux {
    fn id(&self) -> &'static str {
        "tmux"
    }

    fn installed(&self) -> bool {
        self.tmux.is_some()
    }

    fn find(&mut self) {
        let found = Program::find();
        if found.is_some() != self.tmux.is_some() {
            self.version = found.as_ref().map(Program::version).unwrap_or_default();
        }
        self.tmux = found;
    }

    fn info(&self) -> Value {
        json!({ "name": self.socket, "version": self.version })
    }

    fn list(&mut self, ticks: u64, home: &str) -> Result<Listing, String> {
        let tmux = self.program()?.clone();
        if (ticks.is_multiple_of(SERVERS_EVERY) || self.others.is_empty())
            && let Some(dir) = &self.socket_dir
        {
            self.others = server::other_servers(dir, &self.socket);
        }
        let mut panes = tmux.panes(&self.socket)?;
        for other in &self.others {
            // Another server failing (an old tmux, a stale socket) hides only
            // its own panes.
            panes.extend(tmux.panes(other).unwrap_or_default());
        }
        let processes = Processes::list();
        let now = now_ms();
        let now_secs = u64::try_from(now / 1000).unwrap_or(0);

        let mut tracks = HashMap::new();
        let mut terminals = Vec::new();
        let mut live_panes = HashSet::new();
        for pane in panes.iter().filter(|pane| !pane.dead) {
            let ours = pane.server == self.socket;
            let pane_id = if ours {
                pane_of(&pane.session)
            } else {
                String::new()
            };
            if !pane_id.is_empty() {
                live_panes.insert(pane_id.clone());
            }
            let agent = processes
                .agent_under(pane.pid)
                .or_else(|| agents::known(&pane.command));
            let track = agent.map(|_| {
                let previous = self
                    .tracks
                    .get(&pane.address())
                    .copied()
                    .unwrap_or_default();
                let screen = tmux
                    .run(&pane.server, &["capture-pane", "-p", "-J", "-t", &pane.id])
                    .unwrap_or_default();
                let recent = now_secs.saturating_sub(pane.activity) <= RECENT;
                previous.next(agents::read_screen(&screen), recent, now)
            });
            if let Some(track) = track {
                tracks.insert(pane.address(), track);
            }
            terminals.push(entry(pane, agent, track, ours, &pane_id, home));
        }
        self.tracks = tracks;
        self.seen_panes.extend(live_panes.iter().cloned());
        terminals.sort_by_key(sort_key);
        Ok(Listing {
            terminals,
            finished: ended_panes(&self.seen_panes, &live_panes),
        })
    }

    fn owns(&self, address: &str) -> bool {
        server::split_address(address).is_some()
    }

    fn read(&mut self, address: &str) -> Result<String, String> {
        self.on_pane(address, &[&["capture-pane", "-p", "-J", "-S", PEEK_LINES]])
            .map(|text| text.trim_end().to_owned())
    }

    fn send(&mut self, address: &str, text: &str) -> Result<String, String> {
        self.on_pane(
            address,
            &[&["send-keys", "-l", "--", text], &["send-keys", "Enter"]],
        )
    }

    fn keys(&mut self, address: &str, key: &str) -> Result<String, String> {
        let key = tmux_key(key).ok_or_else(|| "that key is not one a peek sends".to_owned())?;
        self.on_pane(address, &[&["send-keys", key]])
    }

    fn interrupt(&mut self, address: &str) -> Result<String, String> {
        self.on_pane(address, &[&["send-keys", "C-c"]])
    }

    fn close_panes(&mut self, panes: &[&str]) -> Result<String, String> {
        let tmux = self.program()?.clone();
        let mut problems = Vec::new();
        for session in tmux.sessions(&self.socket)? {
            let pane = pane_of(&session);
            if !pane.is_empty()
                && panes.contains(&pane.as_str())
                && let Err(failure) = tmux.run(
                    &self.socket,
                    &["kill-session", "-t", &format!("={session}")],
                )
            {
                problems.push(format!("{session}: {}", failure_text(&failure)));
            }
        }
        // A closed pane's id is never given again; forget it.
        for pane in panes {
            self.seen_panes.remove(*pane);
        }
        if problems.is_empty() {
            Ok(String::new())
        } else {
            Err(problems.join("; "))
        }
    }

    fn start(&mut self, pane: &str, key: &str, command: &str) -> Result<Option<String>, String> {
        let Some(found) = self.our_pane(pane)? else {
            return Ok(None);
        };
        let tmux = self.program()?;
        // A session option, set and read through its pane.
        let marked = tmux
            .run(
                &self.socket,
                &["show-options", "-qv", "-t", &found.id, START_OPTION],
            )
            .map_err(|failure| failure_text(&failure))?;
        if !marked.trim().is_empty() {
            return Ok(Some("already started".to_owned()));
        }
        tmux.run(
            &self.socket,
            &["set-option", "-t", &found.id, START_OPTION, key],
        )
        .map_err(|failure| failure_text(&failure))?;
        let address = found.address();
        self.send(&address, command).map(Some)
    }
}

fn failure_text(failure: &Failure) -> String {
    match failure {
        Failure::NoServer => "that tmux server is not running".to_owned(),
        Failure::Failed(error) => error.clone(),
    }
}

/// The tmux name of a key a peek sends.
fn tmux_key(key: &str) -> Option<&'static str> {
    Some(match key {
        "enter" => "Enter",
        "esc" => "Escape",
        "up" => "Up",
        "down" => "Down",
        "tab" => "Tab",
        _ => return None,
    })
}

/// The pane id in the session name of one of this app's panes:
/// `pane_p12+term` (or `pane_p12+term~0123…`) is `p12`; any other is `""`.
fn pane_of(session: &str) -> String {
    let Some((pane, rest)) = session
        .strip_prefix(PANE_SESSION_PREFIX)
        .and_then(|rest| rest.split_once(PANE_SESSION_TERMINAL))
    else {
        return String::new();
    };
    let plain = !pane.is_empty() && pane.bytes().all(|b| b.is_ascii_alphanumeric());
    if plain && (rest.is_empty() || rest.starts_with('~')) {
        pane.to_owned()
    } else {
        String::new()
    }
}

/// One pane as the mods read it.
fn entry(
    pane: &Pane,
    agent: Option<&str>,
    track: Option<Track>,
    ours: bool,
    pane_id: &str,
    home: &str,
) -> Value {
    let track = track.unwrap_or_default();
    let runs = agent.unwrap_or(&pane.command);
    let (key, program) = crate::programs::program_of(runs);
    json!({
        "address": pane.address(),
        "target": if ours { pane.id.clone() } else { pane.target() },
        "backend": "tmux",
        "name": "",
        "cwd": tilde(&pane.cwd, home),
        "program": program,
        "program_key": key,
        "agent": runs,
        "is_agent": agent.is_some(),
        "status": if agent.is_some() { track.status.as_str() } else { "" },
        "status_ms": if agent.is_some() { track.since } else { 0 },
        "parent": "",
        "turn": track.turn,
        "ours": ours,
        "pane": pane_id,
    })
}

/// Ours first, then by server and pane number.
fn sort_key(terminal: &Value) -> (bool, String, u64) {
    let address = terminal["address"].as_str().unwrap_or("");
    let (server, number) = address.rsplit_once(":%").unwrap_or((address, ""));
    (
        terminal["ours"] != Value::Bool(true),
        server.to_owned(),
        number.parse().unwrap_or(u64::MAX),
    )
}

/// The panes of this app seen running in this run whose shell is gone (its
/// session ended). Enter in the pane starts a new one, under the same name.
fn ended_panes(seen: &HashSet<String>, live: &HashSet<String>) -> Vec<String> {
    let mut ended: Vec<String> = seen.difference(live).cloned().collect();
    ended.sort();
    ended
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests;
