//! The programs this app knows by name: the coding agents the "+" menu can
//! start, with whether each is installed here, and how to name and draw
//! what herdr-core says runs in a terminal (`agent`: `zsh`, `claude-code`).

use {
    serde_json::{Value, json},
    std::{
        ffi::OsStr,
        path::{Path, PathBuf},
    },
};

/// A program the app shows by name and icon.
#[derive(Debug, PartialEq)]
pub(crate) struct Program {
    /// Its key, which the Luau draws its icon by: `claude`.
    pub(crate) key: &'static str,
    /// What people call it: `Claude Code`.
    pub(crate) name: &'static str,
    /// The executable that starts it.
    pub(crate) run: &'static str,
    /// The names herdr-core may report it as.
    matches: &'static [&'static str],
}

/// A program by its key, name, executable and the names herdr-core uses.
const fn program(
    key: &'static str,
    name: &'static str,
    run: &'static str,
    matches: &'static [&'static str],
) -> Program {
    Program {
        key,
        name,
        run,
        matches,
    }
}

/// The agents the "+" menu offers, in the order it shows them.
pub(crate) const STARTABLE: &[Program] = &[
    program(
        "claude",
        "Claude Code",
        "claude",
        &["claude", "claude-code"],
    ),
    program("codex", "Codex", "codex", &["codex"]),
    program("opencode", "OpenCode", "opencode", &["opencode"]),
    program(
        "copilot",
        "Copilot",
        "copilot",
        &["copilot", "github-copilot"],
    ),
    program("gemini", "Gemini", "gemini", &["gemini"]),
    program("pi", "Pi", "pi", &["pi"]),
    program(
        "cursor",
        "Cursor",
        "cursor-agent",
        &["cursor", "cursor-agent"],
    ),
];

/// Agents shown by name and icon that the menu does not start.
const KNOWN: &[Program] = &[
    // keep-sorted start
    program("amp", "Amp", "amp", &["amp"]),
    program("droid", "Droid", "droid", &["droid"]),
    program("grok", "Grok", "grok", &["grok", "grok-build"]),
    program("kiro", "Kiro", "kiro-cli", &["kiro", "kiro-cli"]),
    program("qwen", "Qwen Code", "qwen", &["qwen"]),
    // keep-sorted end
];

/// Programs herdr-core reports for a terminal that runs only its shell.
const SHELLS: &[&str] = &[
    // keep-sorted start
    "bash", "csh", "dash", "elvish", "fish", "ksh", "nu", "pwsh", "sh", "tcsh", "xonsh",
    "zsh",
    // keep-sorted end
];

/// Folders under the home folder installers put programs in, looked in
/// after `PATH`: an app opened from the Dock has a short `PATH`.
const HOME_BINS: &[&str] = &[
    ".local/bin",
    ".cargo/bin",
    ".bun/bin",
    ".npm-global/bin",
    ".volta/bin",
    ".opencode/bin",
    ".claude/local",
];

/// Folders package managers put programs in, looked in last.
const SYSTEM_BINS: &[&str] = &["/opt/homebrew/bin", "/usr/local/bin"];

/// The startable agent `key`.
pub(crate) fn startable(key: &str) -> Option<&'static Program> {
    STARTABLE.iter().find(|program| program.key == key)
}

/// What a terminal runs, from herdr-core's `agent`: its key (`claude`,
/// `shell`, or `""` for a program the app does not know) and its name
/// (`Claude Code`, `zsh`, or `agent` itself).
pub(crate) fn program_of(agent: &str) -> (&'static str, String) {
    if let Some(program) = STARTABLE
        .iter()
        .chain(KNOWN)
        .find(|program| program.matches.contains(&agent))
    {
        return (program.key, program.name.to_owned());
    }
    if SHELLS.contains(&agent) {
        return ("shell", agent.to_owned());
    }
    ("", agent.to_owned())
}

/// `name` as one of the names a coding agent runs under (`claude`,
/// `cursor-agent`): how the tmux backend finds agents among processes.
#[cfg(feature = "tmux")]
pub(crate) fn agent_named(name: &str) -> Option<&'static str> {
    STARTABLE
        .iter()
        .chain(KNOWN)
        .flat_map(|program| program.matches.iter().copied())
        .find(|known| *known == name)
}

/// Where `name` is: on `path` (a `PATH` value), else in the usual folders
/// under `home`, else in the package managers' folders. On Windows only on
/// `path`: an app there gets the user's whole `PATH`, and the shell finds
/// what is on it by name, so no path ever needs quoting for it.
pub(crate) fn find_executable(
    name: &str,
    path: Option<&OsStr>,
    home: Option<&Path>,
) -> Option<PathBuf> {
    let on_path = path
        .into_iter()
        .flat_map(std::env::split_paths)
        .filter(|dir| !dir.as_os_str().is_empty());
    let fallbacks = !cfg!(windows);
    let in_home = home
        .filter(|_| fallbacks)
        .into_iter()
        .flat_map(|home| HOME_BINS.iter().map(move |dir| home.join(dir)));
    let in_system = SYSTEM_BINS.iter().filter(|_| fallbacks).map(PathBuf::from);
    on_path
        .chain(in_home)
        .chain(in_system)
        .flat_map(|dir| executable_names(name).map(move |file| dir.join(file)))
        .find(|candidate| runnable(candidate))
}

/// A file the shell can run: on Unix one with an execute bit set, so a
/// stray file of an agent's name does not offer an agent that won't start.
fn runnable(candidate: &Path) -> bool {
    let Ok(metadata) = candidate.metadata() else {
        return false;
    };
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        metadata.is_file() && metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        metadata.is_file()
    }
}

/// The file names `name` may have: on Windows with its extensions.
fn executable_names(name: &str) -> impl Iterator<Item = String> + '_ {
    let extensions: &[&str] = if cfg!(windows) {
        &[".exe", ".cmd", ".bat", ""]
    } else {
        &[""]
    };
    extensions
        .iter()
        .map(move |extension| format!("{name}{extension}"))
}

/// Where `name` is on this machine.
pub(crate) fn locate(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH");
    let home = std::env::var_os("HOME").map(PathBuf::from);
    find_executable(name, path.as_deref(), home.as_deref())
}

/// The startable agents, with whether each is installed:
/// `{ { key, name, installed } }`, in the menu's order.
pub(crate) fn installed() -> Value {
    installed_with(|name| locate(name).is_some())
}

fn installed_with(found: impl Fn(&str) -> bool) -> Value {
    STARTABLE
        .iter()
        .map(|program| {
            json!({
                "key": program.key,
                "name": program.name,
                "installed": found(program.run),
            })
        })
        .collect()
}

/// What to type in a shell to start `program`, found at `found`: its name
/// when the shell finds it on `path`, else the whole path, quoted for a
/// POSIX shell. Windows finds agents only on `PATH` (see
/// [`find_executable`]), so there it is always the name.
pub(crate) fn command_line(program: &Program, found: &Path, path: Option<&OsStr>) -> String {
    let on_path = cfg!(windows)
        || found.parent().is_some_and(|dir| {
            path.into_iter()
                .flat_map(std::env::split_paths)
                .any(|entry| entry == dir)
        });
    if on_path {
        return program.run.to_owned();
    }
    let text = found.to_string_lossy();
    if text
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || "/._-~+".contains(c))
    {
        text.into_owned()
    } else {
        format!("'{}'", text.replace('\'', r"'\''"))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use {
        super::*,
        std::{collections::HashSet, fs},
    };

    /// Writes a program file at `path`, runnable or not.
    fn program_file(path: &Path, runnable: bool) {
        fs::write(path, "").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            let mode = if runnable { 0o755 } else { 0o644 };
            fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
        }
        #[cfg(not(unix))]
        let _ = runnable;
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "herdr-moonframe-programs-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn keys_are_unique_and_every_agent_is_startable_by_key() {
        let mut keys = HashSet::new();
        for program in STARTABLE.iter().chain(KNOWN) {
            assert!(keys.insert(program.key), "{} twice", program.key);
            assert!(!program.run.is_empty());
        }
        for program in STARTABLE {
            assert_eq!(startable(program.key), Some(program));
        }
        assert_eq!(startable("amp"), None);
        assert_eq!(startable("rm -rf ~"), None);
    }

    #[test]
    fn terminals_are_named_after_their_program() {
        assert_eq!(program_of("claude-code"), ("claude", "Claude Code".into()));
        assert_eq!(program_of("cursor-agent"), ("cursor", "Cursor".into()));
        assert_eq!(program_of("github-copilot"), ("copilot", "Copilot".into()));
        assert_eq!(program_of("qwen"), ("qwen", "Qwen Code".into()));
        assert_eq!(program_of("zsh"), ("shell", "zsh".into()));
        assert_eq!(program_of("htop"), ("", "htop".into()));
        assert_eq!(program_of(""), ("", String::new()));
    }

    #[test]
    fn executables_are_found_on_path_first() {
        let root = scratch("path");
        let (first, second) = (root.join("first"), root.join("second"));
        fs::create_dir_all(&first).unwrap();
        fs::create_dir_all(&second).unwrap();
        let name = format!("herdr-test-agent-{}", std::process::id());
        let file = |dir: &Path| dir.join(executable_names(&name).next().unwrap());
        let path = std::env::join_paths([&first, &second]).unwrap();
        assert_eq!(find_executable(&name, Some(&path), None), None);
        program_file(&file(&second), true);
        assert_eq!(
            find_executable(&name, Some(&path), None),
            Some(file(&second))
        );
        program_file(&file(&first), true);
        assert_eq!(
            find_executable(&name, Some(&path), None),
            Some(file(&first))
        );
        // A folder of that name is not a program.
        let folder = format!("{name}-dir");
        fs::create_dir_all(first.join(executable_names(&folder).next().unwrap())).unwrap();
        assert_eq!(find_executable(&folder, Some(&path), None), None);
        let _ = fs::remove_dir_all(&root);
    }

    #[cfg(not(windows))]
    #[test]
    fn the_usual_folders_are_looked_in_after_path() {
        let root = scratch("home");
        let on_path = root.join("bin");
        let local = root.join("home/.local/bin");
        fs::create_dir_all(&on_path).unwrap();
        fs::create_dir_all(&local).unwrap();
        let home = root.join("home");
        let name = format!("herdr-test-agent-{}", std::process::id());
        let path = std::env::join_paths([&on_path]).unwrap();
        program_file(&local.join(&name), true);
        assert_eq!(
            find_executable(&name, Some(&path), Some(&home)),
            Some(local.join(&name))
        );
        program_file(&on_path.join(&name), true);
        assert_eq!(
            find_executable(&name, Some(&path), Some(&home)),
            Some(on_path.join(&name))
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[cfg(unix)]
    #[test]
    fn a_file_without_an_execute_bit_is_not_a_program() {
        let root = scratch("mode");
        let name = format!("herdr-test-agent-{}", std::process::id());
        let path = std::env::join_paths([&root]).unwrap();
        program_file(&root.join(&name), false);
        assert_eq!(find_executable(&name, Some(&path), None), None);
        program_file(&root.join(&name), true);
        assert_eq!(
            find_executable(&name, Some(&path), None),
            Some(root.join(&name))
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn the_menu_lists_every_startable_agent_in_order() {
        let listed = installed_with(|run| run == "claude" || run == "cursor-agent");
        let listed = listed.as_array().unwrap();
        let keys: Vec<&str> = listed
            .iter()
            .map(|entry| entry["key"].as_str().unwrap())
            .collect();
        let expected: Vec<&str> = STARTABLE.iter().map(|program| program.key).collect();
        assert_eq!(keys, expected);
        let installed: Vec<&str> = listed
            .iter()
            .filter(|entry| entry["installed"] == true)
            .map(|entry| entry["key"].as_str().unwrap())
            .collect();
        assert_eq!(installed, ["claude", "cursor"]);
        assert_eq!(listed[0]["name"], "Claude Code");
    }

    #[cfg(not(windows))]
    #[test]
    fn the_command_is_the_name_when_the_shell_finds_it() {
        let claude = startable("claude").unwrap();
        let path = std::env::join_paths(["/usr/bin", "/opt/bin"]).unwrap();
        assert_eq!(
            command_line(claude, Path::new("/opt/bin/claude"), Some(&path)),
            "claude"
        );
        assert_eq!(
            command_line(
                claude,
                Path::new("/home/u/.claude/local/claude"),
                Some(&path)
            ),
            "/home/u/.claude/local/claude"
        );
        assert_eq!(
            command_line(claude, Path::new("/home/it's me/bin/claude"), None),
            r"'/home/it'\''s me/bin/claude'"
        );
    }

    #[cfg(windows)]
    #[test]
    fn on_windows_the_command_is_always_the_name() {
        let claude = startable("claude").unwrap();
        assert_eq!(
            command_line(claude, Path::new(r"C:\Users\me\bin\claude.exe"), None),
            "claude"
        );
    }
}
