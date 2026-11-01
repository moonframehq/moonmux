//! Which panes run a coding agent, and what that agent is doing.
//!
//! tmux knows a pane's shell and its foreground program, not agents. So the
//! agent is found in the processes under the pane's shell (`ps`), and its
//! state is read off its screen: agents print a hint to stop them while
//! they work (`esc to interrupt`) and ask before acting (`Do you want
//! to…`). An agent that stops working is Done until it starts again.
//! Screens are untrusted text: they only ever choose among these states.

use {
    crate::core::new_command,
    std::{collections::HashMap, process::Stdio},
};

/// Runtimes an agent may run under, its script named next (`node
/// …/bin/codex`).
const RUNTIMES: &[&str] = &[
    // keep-sorted start
    "bun", "deno", "node", "python", "python3",
    // keep-sorted end
];

/// Lines at the foot of the screen that are read for a state.
const FOOT_LINES: usize = 15;
/// How many processes under one pane are looked through, at most.
const MAX_DESCENDANTS: usize = 256;

/// What agents print while they work.
const WORKING: &[&str] = &[
    // keep-sorted start
    "ctrl+c to interrupt",
    "ctrl-c to interrupt",
    "esc to cancel",
    "esc to interrupt",
    "esc to stop",
    "esc interrupt",
    // keep-sorted end
];

/// What agents print when they wait on you.
const BLOCKED: &[&str] = &[
    // keep-sorted start
    "(y/n)",
    "(yes/no)",
    "[y/n]",
    "allow always",
    "allow once",
    "do you want to",
    "don't ask again",
    "press enter to continue",
    "would you like to",
    // keep-sorted end
];

/// Every process: its parent and its command line, by pid.
#[derive(Default)]
pub(crate) struct Processes {
    children: HashMap<u32, Vec<u32>>,
    args: HashMap<u32, String>,
}

impl Processes {
    /// What `ps` lists now; nothing where there is no `ps`.
    pub(crate) fn list() -> Self {
        let output = new_command("ps")
            .args(["-A", "-o", "pid=,ppid=,args="])
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output();
        match output {
            Ok(output) if output.status.success() => {
                Self::parse(&String::from_utf8_lossy(&output.stdout))
            }
            _ => Self::default(),
        }
    }

    fn parse(text: &str) -> Self {
        let mut processes = Self::default();
        for line in text.lines() {
            let mut fields = line.split_whitespace();
            let (Some(pid), Some(ppid)) = (fields.next(), fields.next()) else {
                continue;
            };
            let (Ok(pid), Ok(ppid)) = (pid.parse::<u32>(), ppid.parse::<u32>()) else {
                continue;
            };
            processes.children.entry(ppid).or_default().push(pid);
            processes
                .args
                .insert(pid, fields.collect::<Vec<_>>().join(" "));
        }
        processes
    }

    /// The agent running as `pid` (a pane's first program) or under it, if
    /// any: the one nearest it.
    pub(crate) fn agent_under(&self, pid: u32) -> Option<&'static str> {
        let mut queue = std::collections::VecDeque::from([pid]);
        let mut looked = 0;
        while let Some(next) = queue.pop_front() {
            looked += 1;
            if looked > MAX_DESCENDANTS {
                break;
            }
            if let Some(agent) = self.args.get(&next).and_then(|args| agent_in(args)) {
                return Some(agent);
            }
            queue.extend(self.children.get(&next).into_iter().flatten().copied());
        }
        None
    }
}

/// The last part of a path: `/opt/bin/codex` is `codex`.
fn base(word: &str) -> &str {
    word.rsplit('/').next().unwrap_or(word)
}

/// The agent a command line runs: its program, or the script a runtime
/// runs (`node /usr/lib/node_modules/@openai/codex/bin/codex.js`).
pub(crate) fn agent_in(args: &str) -> Option<&'static str> {
    let mut words = args.split_whitespace();
    let program = base(words.next()?).trim_start_matches('-');
    if let Some(agent) = known(program) {
        return Some(agent);
    }
    if RUNTIMES.contains(&program) {
        let script = words.find(|word| !word.starts_with('-'))?;
        let script = base(script);
        let script = script
            .strip_suffix(".js")
            .or_else(|| script.strip_suffix(".mjs"))
            .or_else(|| script.strip_suffix(".py"))
            .unwrap_or(script);
        return known(script);
    }
    None
}

/// `name` as one of the names a coding agent runs under (programs.rs).
pub(crate) fn known(name: &str) -> Option<&'static str> {
    crate::programs::agent_named(name)
}

/// What the foot of an agent's screen says it is doing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Seen {
    Working,
    Blocked,
    Quiet,
}

pub(crate) fn read_screen(screen: &str) -> Seen {
    let foot: Vec<String> = screen
        .lines()
        .rev()
        .filter(|line| !line.trim().is_empty())
        .take(FOOT_LINES)
        .map(str::to_lowercase)
        .collect();
    let says = |phrases: &[&str]| {
        foot.iter()
            .any(|line| phrases.iter().any(|phrase| line.contains(phrase)))
    };
    if says(BLOCKED) {
        Seen::Blocked
    } else if says(WORKING) {
        Seen::Working
    } else {
        Seen::Quiet
    }
}

/// An agent's state, as the mods read it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Status {
    #[default]
    Idle,
    Working,
    Blocked,
    /// It stopped working, and has not started again.
    Done,
}

impl Status {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Working => "working",
            Self::Blocked => "blocked",
            Self::Done => "done",
        }
    }
}

/// What is remembered of one agent between ticks: its state, since when
/// (ms since the epoch; 0 for an idle agent never seen otherwise), and how
/// many times it finished (a `done` the user has looked at shows as idle
/// until the next one).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Track {
    pub(crate) status: Status,
    pub(crate) since: i64,
    pub(crate) turn: i64,
}

impl Track {
    /// The next state, from what its screen shows `now` (ms since the
    /// epoch). `recent` is whether it printed within the last moments: a
    /// working agent between two hints stays working.
    pub(crate) fn next(self, seen: Seen, recent: bool, now: i64) -> Self {
        let status = match seen {
            Seen::Blocked => Status::Blocked,
            Seen::Working => Status::Working,
            Seen::Quiet => match self.status {
                Status::Working if recent => Status::Working,
                Status::Working | Status::Done => Status::Done,
                Status::Blocked | Status::Idle => Status::Idle,
            },
        };
        let turn = if status == Status::Done && self.status != Status::Done {
            self.turn + 1
        } else {
            self.turn
        };
        let since = if status == self.status {
            self.since
        } else {
            now
        };
        Self {
            status,
            since,
            turn,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn agents_are_found_by_program_or_script() {
        assert_eq!(agent_in("claude --resume"), Some("claude"));
        assert_eq!(agent_in("/opt/homebrew/bin/codex"), Some("codex"));
        assert_eq!(
            agent_in("node /usr/lib/node_modules/@openai/codex/bin/codex.js"),
            Some("codex")
        );
        assert_eq!(agent_in("node --no-warnings /x/bin/gemini"), Some("gemini"));
        assert_eq!(agent_in("-zsh"), None);
        assert_eq!(agent_in("node server.js"), None);
        assert_eq!(agent_in("vim claude"), None);
        assert_eq!(agent_in(""), None);
    }

    #[test]
    fn the_agent_under_a_shell_is_found() {
        let ps = "  10     1 -zsh\n  11    10 node /x/bin/codex.js\n  12    11 /bin/sh -c ls\n  20     1 claude\n";
        let processes = Processes::parse(ps);
        assert_eq!(processes.agent_under(10), Some("codex"));
        assert_eq!(processes.agent_under(12), None);
        assert_eq!(processes.agent_under(99), None);
        // A pane started as the agent itself.
        assert_eq!(processes.agent_under(20), Some("claude"));
    }

    #[test]
    fn a_cycle_in_ps_ends() {
        let processes = Processes::parse("1 2 zsh\n2 1 zsh\n");
        assert_eq!(processes.agent_under(1), None);
    }

    #[test]
    fn screens_say_working_blocked_or_nothing() {
        assert_eq!(
            read_screen("✻ Thinking… (esc to interrupt)\n> "),
            Seen::Working
        );
        assert_eq!(
            read_screen("Do you want to make this edit?\n❯ 1. Yes\n  2. No\n"),
            Seen::Blocked
        );
        assert_eq!(read_screen("> \n\n\n"), Seen::Quiet);
        // Only the foot of the screen counts.
        let old = format!("esc to interrupt\n{}", "line\n".repeat(FOOT_LINES + 1));
        assert_eq!(read_screen(&old), Seen::Quiet);
    }

    #[test]
    fn working_then_quiet_is_done_once() {
        let idle = Track::default();
        let working = idle.next(Seen::Working, true, 10);
        assert_eq!(working.status, Status::Working);
        assert_eq!(working.since, 10);
        // Printing between two hints, it still works, since it started.
        assert_eq!(working.next(Seen::Quiet, true, 20), working);
        let done = working.next(Seen::Quiet, false, 30);
        assert_eq!(
            done,
            Track {
                status: Status::Done,
                since: 30,
                turn: 1
            }
        );
        assert_eq!(done.next(Seen::Quiet, false, 40), done);
        let again = done
            .next(Seen::Working, true, 50)
            .next(Seen::Quiet, false, 60);
        assert_eq!(
            again,
            Track {
                status: Status::Done,
                since: 60,
                turn: 2
            }
        );
    }

    #[test]
    fn blocked_until_answered() {
        let blocked = Track::default().next(Seen::Blocked, true, 5);
        assert_eq!(blocked.status, Status::Blocked);
        assert_eq!(blocked.next(Seen::Quiet, true, 6).status, Status::Idle);
        assert_eq!(blocked.next(Seen::Working, true, 6).status, Status::Working);
        // An agent first seen idle has no time to show.
        assert_eq!(
            Track::default().next(Seen::Quiet, true, 7),
            Track::default()
        );
    }
}
