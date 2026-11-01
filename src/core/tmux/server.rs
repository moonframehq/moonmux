//! Talking to tmux: the `tmux` program, the servers it runs for this user,
//! and what `list-panes` says about each pane.
//!
//! Every server is reached by name (`tmux -L <name>`), the user's own one
//! too (`-L default`), so a `$TMUX` inherited from a surrounding tmux never
//! picks the server for us.

use {
    crate::core::new_command,
    std::{
        path::{Path, PathBuf},
        process::Stdio,
    },
};

/// The server tmux uses when given no `-L`.
pub(crate) const DEFAULT: &str = "default";
/// Servers looked at besides the app's own, at most: each costs a
/// `list-panes` every tick.
const MAX_SERVERS: usize = 8;
/// What `list-panes` prints for each pane, one field per tab.
const PANE_FORMAT: &str = "#{pane_id}\t#{session_name}\t#{window_index}\t#{pane_index}\t\
                           #{pane_pid}\t#{pane_current_command}\t#{pane_current_path}\t\
                           #{pane_dead}\t#{window_activity}";

/// The `tmux` found on this machine.
#[derive(Clone, Debug)]
pub(crate) struct Program {
    path: PathBuf,
}

/// What a tmux command printed, or why it failed.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Failure {
    /// No server runs under that name: nothing to list, not an error.
    NoServer,
    Failed(String),
}

impl Program {
    /// `$TMUX_BIN`, else `tmux` on `PATH` or where package managers put it
    /// (an app opened from the Dock has a short `PATH`).
    pub(crate) fn find() -> Option<Self> {
        if let Some(bin) = std::env::var_os("TMUX_BIN").map(PathBuf::from)
            && bin.is_file()
        {
            return Some(Self { path: bin });
        }
        let home = std::env::var_os("HOME").map(PathBuf::from);
        crate::programs::locate("tmux")
            .or_else(|| {
                ["/usr/bin/tmux", "/run/current-system/sw/bin/tmux"]
                    .iter()
                    .map(PathBuf::from)
                    .chain(home.map(|home| home.join(".nix-profile/bin/tmux")))
                    .find(|candidate| candidate.is_file())
            })
            .map(|path| Self { path })
    }

    #[cfg(test)]
    pub(crate) fn at(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// `tmux -V`: `tmux 3.5a` is `3.5a`.
    pub(crate) fn version(&self) -> String {
        new_command(&self.path)
            .arg("-V")
            .stdin(Stdio::null())
            .output()
            .ok()
            .filter(|output| output.status.success())
            .map(|output| {
                let said = String::from_utf8_lossy(&output.stdout);
                said.trim().trim_start_matches("tmux ").to_owned()
            })
            .unwrap_or_default()
    }

    /// Runs `tmux -L server args...` and returns what it printed.
    pub(crate) fn run(&self, server: &str, args: &[&str]) -> Result<String, Failure> {
        let output = new_command(&self.path)
            .args(["-L", server])
            .args(args)
            .stdin(Stdio::null())
            .output()
            .map_err(|error| Failure::Failed(format!("cannot run tmux: {error}")))?;
        if output.status.success() {
            return Ok(String::from_utf8_lossy(&output.stdout).into_owned());
        }
        let said = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        if no_server(&said) {
            Err(Failure::NoServer)
        } else if said.is_empty() {
            Err(Failure::Failed(format!(
                "tmux {} failed",
                args.first().unwrap_or(&"")
            )))
        } else {
            Err(Failure::Failed(said))
        }
    }

    /// Every pane on `server`; none when it does not run.
    pub(crate) fn panes(&self, server: &str) -> Result<Vec<Pane>, String> {
        match self.run(server, &["list-panes", "-a", "-F", PANE_FORMAT]) {
            Ok(text) => Ok(text
                .lines()
                .filter_map(|line| Pane::parse(server, line))
                .collect()),
            Err(Failure::NoServer) => Ok(Vec::new()),
            Err(Failure::Failed(error)) => Err(error),
        }
    }

    /// The names of the sessions on `server`.
    pub(crate) fn sessions(&self, server: &str) -> Result<Vec<String>, String> {
        match self.run(server, &["list-sessions", "-F", "#{session_name}"]) {
            Ok(text) => Ok(text.lines().map(str::to_owned).collect()),
            Err(Failure::NoServer) => Ok(Vec::new()),
            Err(Failure::Failed(error)) => Err(error),
        }
    }
}

/// What tmux says when no server listens on the socket.
fn no_server(said: &str) -> bool {
    said.starts_with("no server running")
        || said.starts_with("error connecting to")
        || said.contains("No such file or directory")
}

/// One pane, as `list-panes` describes it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Pane {
    pub(crate) server: String,
    /// tmux's id for it, `%12`: unique on its server while it lives.
    pub(crate) id: String,
    pub(crate) session: String,
    pub(crate) window: String,
    pub(crate) index: String,
    pub(crate) pid: u32,
    /// The foreground program, as tmux names it.
    pub(crate) command: String,
    pub(crate) cwd: String,
    pub(crate) dead: bool,
    /// When its window last printed something, in seconds since the epoch.
    pub(crate) activity: u64,
}

impl Pane {
    fn parse(server: &str, line: &str) -> Option<Self> {
        let fields: Vec<&str> = line.split('\t').collect();
        let [
            id,
            session,
            window,
            index,
            pid,
            command,
            cwd,
            dead,
            activity,
        ] = fields[..]
        else {
            return None;
        };
        is_pane_id(id).then(|| Self {
            server: server.to_owned(),
            id: id.to_owned(),
            session: session.to_owned(),
            window: window.to_owned(),
            index: index.to_owned(),
            pid: pid.parse().unwrap_or(0),
            command: command.to_owned(),
            cwd: cwd.to_owned(),
            dead: dead == "1",
            activity: activity.parse().unwrap_or(0),
        })
    }

    /// How the mods name it: `server:%12`.
    pub(crate) fn address(&self) -> String {
        format!("{}:{}", self.server, self.id)
    }

    /// Where a user finds it in tmux: `work:1.0`, after its server's name
    /// when that is not the default one.
    pub(crate) fn target(&self) -> String {
        let target = format!("{}:{}.{}", self.session, self.window, self.index);
        if self.server == DEFAULT {
            target
        } else {
            format!("{} {target}", self.server)
        }
    }
}

/// `%12`: what tmux names a pane.
pub(crate) fn is_pane_id(id: &str) -> bool {
    id.strip_prefix('%')
        .is_some_and(|digits| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
}

/// A name `tmux -L` takes: a plain file name in the socket folder.
pub(crate) fn is_server_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
        && !name.starts_with('.')
}

/// Splits an address, `server:%12`, checking both halves.
pub(crate) fn split_address(address: &str) -> Option<(&str, &str)> {
    let (server, pane) = address.rsplit_once(':')?;
    (is_server_name(server) && is_pane_id(pane)).then_some((server, pane))
}

/// The folder tmux keeps this user's sockets in: `$TMUX_TMPDIR` (else
/// `/tmp`), then `tmux-<uid>`.
pub(crate) fn socket_dir() -> Option<PathBuf> {
    let uid = new_command("id")
        .arg("-u")
        .stdin(Stdio::null())
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .filter(|uid| !uid.is_empty() && uid.bytes().all(|b| b.is_ascii_digit()))?;
    let base = std::env::var_os("TMUX_TMPDIR")
        .map(PathBuf::from)
        .filter(|dir| dir.is_dir())
        .unwrap_or_else(|| PathBuf::from("/tmp"));
    Some(base.join(format!("tmux-{uid}")))
}

/// The servers with a socket in `dir`, apart from `ours`, sorted, the
/// default one first; at most [`MAX_SERVERS`].
pub(crate) fn other_servers(dir: &Path, ours: &str) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(is_socket))
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| name != ours && is_server_name(name))
        .collect();
    names.sort_by_key(|name| (name != DEFAULT, name.clone()));
    names.truncate(MAX_SERVERS);
    names
}

#[cfg(unix)]
fn is_socket(kind: std::fs::FileType) -> bool {
    use std::os::unix::fs::FileTypeExt as _;
    kind.is_socket()
}

#[cfg(not(unix))]
fn is_socket(_kind: std::fs::FileType) -> bool {
    false
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn a_pane_line_parses() {
        let line = "%7\twork\t1\t0\t4242\tzsh\t/home/u/x\t0\t1700000000";
        let pane = Pane::parse("default", line).unwrap();
        assert_eq!(pane.id, "%7");
        assert_eq!(pane.pid, 4242);
        assert_eq!(pane.cwd, "/home/u/x");
        assert!(!pane.dead);
        assert_eq!(pane.activity, 1_700_000_000);
        assert_eq!(pane.address(), "default:%7");
        assert_eq!(pane.target(), "work:1.0");
        let other = Pane::parse("scratch", line).unwrap();
        assert_eq!(other.target(), "scratch work:1.0");
    }

    #[test]
    fn a_malformed_line_is_skipped() {
        assert!(Pane::parse("default", "").is_none());
        assert!(Pane::parse("default", "%7\twork").is_none());
        assert!(Pane::parse("default", "7\tw\t1\t0\t1\tzsh\t/\t0\t0").is_none());
    }

    #[test]
    fn addresses_are_checked() {
        assert_eq!(split_address("default:%3"), Some(("default", "%3")));
        assert_eq!(
            split_address("herdr-moonframe:%12"),
            Some(("herdr-moonframe", "%12"))
        );
        assert_eq!(split_address("default:3"), None);
        assert_eq!(split_address("../x:%3"), None);
        assert_eq!(split_address("a b:%3"), None);
        assert_eq!(split_address("%3"), None);
        assert_eq!(split_address(":%3"), None);
    }

    #[test]
    fn missing_servers_are_not_errors() {
        assert!(no_server(
            "no server running on /tmp/tmux-501/herdr-moonframe"
        ));
        assert!(no_server(
            "error connecting to /tmp/tmux-501/x (No such file or directory)"
        ));
        assert!(!no_server("can't find pane: %9"));
    }

    #[cfg(unix)]
    #[test]
    fn only_sockets_count_as_servers() {
        use std::os::unix::net::UnixListener;
        let dir =
            std::env::temp_dir().join(format!("herdr-moonframe-sockets-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let _a = UnixListener::bind(dir.join("work")).unwrap();
        let _b = UnixListener::bind(dir.join(DEFAULT)).unwrap();
        let _c = UnixListener::bind(dir.join("herdr-moonframe")).unwrap();
        std::fs::write(dir.join("notes"), "").unwrap();
        let found = other_servers(&dir, "herdr-moonframe");
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(found, vec![DEFAULT.to_owned(), "work".to_owned()]);
        assert!(other_servers(&dir.join("missing"), "x").is_empty());
    }

    #[test]
    fn a_missing_tmux_fails_to_run() {
        let tmux = Program::at("/nonexistent/tmux");
        assert!(matches!(
            tmux.run(DEFAULT, &["ls"]),
            Err(Failure::Failed(_))
        ));
        assert_eq!(tmux.version(), "");
    }
}
