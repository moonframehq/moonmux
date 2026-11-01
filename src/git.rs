//! What the `folder` and `changes` providers share: running git with its
//! output bounded, reading `git status`, and checking who may ask.
//!
//! Both follow the folder a mod names (the focused pane's), which
//! Moonframe's own `files` and `git` providers cannot: theirs is fixed when
//! they start. Every read here writes nothing under `.git` (no index lock,
//! no fsmonitor cookie), and nothing a folder holds is trusted: names and
//! text are data, and paths are checked to stay inside the folder.

use {
    crate::core::new_command,
    moonframe::Data,
    std::{
        ffi::OsString,
        io::Read,
        path::{Component, Path, PathBuf},
        process::Stdio,
    },
};

/// Options for reads that leave the repository as it was.
const READ: &[&str] = &[
    "--no-optional-locks",
    // A path is the file of that name, never a pattern: `a*b.txt` is one
    // file.
    "--literal-pathspecs",
    "-c",
    "core.fsmonitor=false",
    "-c",
    "diff.autoRefreshIndex=false",
    "-c",
    "core.quotepath=false",
    // No program a repository's own config names runs for a look: no
    // external diff (textconv is refused per command, `--no-textconv`).
    "-c",
    "diff.external=",
];

/// What a bounded git read printed.
pub(crate) struct Output {
    pub(crate) bytes: Vec<u8>,
    /// It printed more than the bound; `bytes` is the start.
    pub(crate) truncated: bool,
}

/// Runs `git -C root <args>` for a read, keeping at most `limit` bytes of
/// what it prints: a huge diff or status costs the bound, not the file.
///
/// The folder is whatever the focused pane shows, a repository cloned from
/// anywhere included, so its own config must not run programs: the filter
/// drivers it names (`filter.<x>.clean`, `smudge`, `process`, which `git
/// status` runs on changed files) are emptied, which git takes as none.
/// When they cannot all be found, nothing runs. `status` and `diff` never
/// go into submodules, whose own config would name filters of its own:
/// `--ignore-submodules=all` on the command line wins over any
/// `submodule.<x>.ignore` the repository sets.
pub(crate) fn read(root: &Path, args: &[&str], limit: usize) -> Result<Output, String> {
    let keys = filter_keys(root)?;
    let mut args = args.to_vec();
    if matches!(args.first(), Some(&("status" | "diff"))) {
        args.insert(1, "--ignore-submodules=all");
    }
    let (output, success) = run(root, &keys, &args, limit)?;
    if !output.truncated && !success {
        let name = args.first().copied().unwrap_or("git");
        return Err(format!("git {name} failed"));
    }
    Ok(output)
}

/// Bytes of config names read, and filter commands emptied, at most:
/// past either the read is refused rather than left partly guarded.
const FILTER_BYTES: usize = 64 * 1024;
const MAX_FILTERS: usize = 256;

/// Every filter driver command key git's config names here, exactly as git
/// prints them. Reading config runs nothing.
fn filter_keys(root: &Path) -> Result<Vec<OsString>, String> {
    let args = [
        "config",
        "-z",
        "--name-only",
        "--get-regexp",
        r"^filter\..*\.(clean|smudge|process)$",
    ];
    let (output, success) = run(root, &[], &args, FILTER_BYTES)?;
    if output.truncated {
        return Err("too many git filters to switch off".to_owned());
    }
    if !success {
        // `git config --get-regexp` exits 1 when nothing matches, and
        // prints nothing; any other failure prints nothing too, so a
        // failed read is told apart by trying again with a name that is
        // always set.
        return if output.bytes.is_empty() && config_readable(root) {
            Ok(Vec::new())
        } else {
            Err("cannot read git's config to switch off its filters".to_owned())
        };
    }
    let keys: Vec<OsString> = output
        .bytes
        .split(|byte| *byte == 0)
        .filter(|key| !key.is_empty())
        .map(os_key)
        .collect::<Option<_>>()
        .ok_or("a git filter has a name that cannot be switched off")?;
    if keys.len() > MAX_FILTERS {
        return Err("too many git filters to switch off".to_owned());
    }
    Ok(keys)
}

/// Whether git can read its config here at all: `--list` succeeds even
/// when it is empty.
fn config_readable(root: &Path) -> bool {
    run(root, &[], &["config", "-z", "--list"], FILTER_BYTES).is_ok_and(|(_, success)| success)
}

/// A config key as git printed it, passed on unchanged; one with a newline
/// (which no config file can hold) is refused.
fn os_key(key: &[u8]) -> Option<OsString> {
    if key.contains(&b'\n') {
        return None;
    }
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        Some(std::ffi::OsStr::from_bytes(key).to_owned())
    }
    #[cfg(not(unix))]
    {
        std::str::from_utf8(key).ok().map(OsString::from)
    }
}

/// Runs git for a read; what it printed, and whether it succeeded.
///
/// `empty` keys are set to `""` through `GIT_CONFIG_COUNT` and
/// `GIT_CONFIG_KEY_<n>`/`GIT_CONFIG_VALUE_<n>`, which take a key whole: no
/// `=` in it is ever split, as `-c key=value` would.
fn run(
    root: &Path,
    empty: &[OsString],
    args: &[&str],
    limit: usize,
) -> Result<(Output, bool), String> {
    let name = args.first().copied().unwrap_or("git");
    let mut command = new_command("git");
    command
        .arg("-C")
        .arg(root)
        .args(READ)
        .args(args)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_CONFIG_COUNT", empty.len().to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        // Never read: a full pipe would stall git while stdout is read.
        .stderr(Stdio::null());
    for (index, key) in empty.iter().enumerate() {
        command
            .env(format!("GIT_CONFIG_KEY_{index}"), key)
            .env(format!("GIT_CONFIG_VALUE_{index}"), "");
    }
    let mut child = command
        .spawn()
        .map_err(|error| format!("git {name}: {error}"))?;
    let mut bytes = Vec::new();
    if let Some(stdout) = child.stdout.take() {
        stdout
            .take(limit as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| format!("git {name}: {error}"))?;
    }
    let truncated = bytes.len() > limit;
    if truncated {
        bytes.truncate(limit);
        // It has more to say; nothing more is wanted.
        let _ = child.kill();
    }
    let status = child
        .wait()
        .map_err(|error| format!("git {name}: {error}"))?;
    Ok((Output { bytes, truncated }, status.success()))
}

/// The top folder of the repository `dir` is in, if any.
pub(crate) fn toplevel(dir: &Path) -> Option<PathBuf> {
    let output = read(dir, &["rev-parse", "--show-toplevel"], 4096).ok()?;
    let text = String::from_utf8(output.bytes).ok()?;
    let top = text.trim_end_matches(['\n', '\r']);
    (!top.is_empty()).then(|| PathBuf::from(top))
}

/// `path` with a leading `~` made the home folder, as `app.core` writes
/// folders under it.
pub(crate) fn expand(path: &str, home: &Path) -> PathBuf {
    if path == "~" {
        home.to_owned()
    } else if let Some(rest) = path.strip_prefix("~/") {
        home.join(rest)
    } else {
        PathBuf::from(path)
    }
}

/// The user's home folder, or nothing when it is unknown.
pub(crate) fn home() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_default()
}

/// The folder a mod named: an absolute path (after `~`) to an existing
/// folder, made canonical so later checks compare like with like.
pub(crate) fn folder(asked: &str) -> Option<PathBuf> {
    let path = expand(asked.trim(), &home());
    if !path.is_absolute() {
        return None;
    }
    let real = path.canonicalize().ok()?;
    real.is_dir().then_some(real)
}

/// Whether `relative` is a plain path inside a folder: no root, no `.` or
/// `..`, no empty parts.
pub(crate) fn plain(relative: &str) -> bool {
    !relative.is_empty()
        && !relative.split('/').any(str::is_empty)
        && Path::new(relative)
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
}

/// `root/relative`, when `relative` is plain and stays inside `root` once
/// links are followed; `root` is canonical.
pub(crate) fn inside(root: &Path, relative: &str) -> Option<PathBuf> {
    if !plain(relative) {
        return None;
    }
    let real = root.join(relative).canonicalize().ok()?;
    real.starts_with(root).then_some(real)
}

/// The names in `[settings] <key>` of provider.toml; none when missing.
pub(crate) fn names(settings: &Data, key: &str) -> Vec<String> {
    let Data::Map(settings) = settings else {
        return Vec::new();
    };
    match settings.get(key) {
        Some(Data::List(names)) => names
            .iter()
            .filter_map(|name| match name {
                Data::String(name) => Some(name.clone()),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// Whether `from` is one of `allowed`: a mod, or an instance of one
/// (`files#right` is the `files` mod's).
pub(crate) fn allowed(allowed: &[String], from: &str) -> bool {
    let name = from.split('#').next().unwrap_or(from);
    !name.contains('/') && allowed.iter().any(|each| each == name)
}

/// One entry of `git status`.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Change {
    /// From the repository's top folder.
    pub(crate) path: String,
    /// `A`, `M`, `D` or `U` (a conflict), as `change-tree` marks them.
    pub(crate) status: char,
    /// Not known to git yet: its lines are the file's.
    pub(crate) untracked: bool,
}

/// The entries of `git status --porcelain=v1 -z --no-renames`, at most
/// `max`; the second value says whether there were more.
pub(crate) fn status(bytes: &[u8], max: usize) -> (Vec<Change>, bool) {
    let mut changes = Vec::new();
    let mut more = false;
    for entry in bytes.split(|byte| *byte == 0) {
        if entry.len() < 4 {
            continue;
        }
        if changes.len() == max {
            more = true;
            break;
        }
        let (xy, path) = (&entry[..2], &entry[3..]);
        let untracked = xy == b"??";
        let conflict = xy.contains(&b'U') || xy == b"AA" || xy == b"DD";
        let status = if conflict {
            'U'
        } else if xy.contains(&b'D') {
            'D'
        } else if untracked || xy.contains(&b'A') {
            'A'
        } else {
            'M'
        };
        changes.push(Change {
            path: String::from_utf8_lossy(path).into_owned(),
            status,
            untracked,
        });
    }
    (changes, more)
}

/// Where `root` sits in its repository whose top is `top`: `""` at the
/// top, else `"sub/dir/"`. Both canonical.
pub(crate) fn prefix(top: &Path, root: &Path) -> Option<String> {
    let top = top.canonicalize().ok()?;
    let rest = root.strip_prefix(&top).ok()?;
    let text = rest.to_string_lossy().replace('\\', "/");
    Some(if text.is_empty() {
        text
    } else {
        format!("{text}/")
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
pub(crate) mod tests {
    use super::*;

    #[test]
    fn a_tilde_is_the_home_folder() {
        let home = Path::new("/home/me");
        assert_eq!(expand("~", home), PathBuf::from("/home/me"));
        assert_eq!(expand("~/code/x", home), PathBuf::from("/home/me/code/x"));
        assert_eq!(expand("/srv/x", home), PathBuf::from("/srv/x"));
        assert_eq!(expand("~other", home), PathBuf::from("~other"));
    }

    #[test]
    fn only_plain_paths_go_inside_a_folder() {
        assert!(plain("src"));
        assert!(plain("src/main.rs"));
        assert!(plain(".github/workflows"));
        assert!(!plain(""));
        assert!(!plain("/etc/passwd"));
        assert!(!plain("../x"));
        assert!(!plain("src/../../x"));
        assert!(!plain("./src"));
        assert!(!plain("src//main.rs"));
        assert!(!plain("src/"));
    }

    #[test]
    fn a_link_out_of_the_folder_stays_out() {
        let dir = tempdir();
        let root = dir.join("root");
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src/a.txt"), "a").unwrap();
        std::fs::write(dir.join("secret"), "s").unwrap();
        let root = root.canonicalize().unwrap();
        assert!(inside(&root, "src/a.txt").is_some());
        assert!(inside(&root, "../secret").is_none());
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(dir.join("secret"), root.join("link")).unwrap();
            assert!(inside(&root, "link").is_none());
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn instances_count_as_their_mod() {
        let names = names(&Data::map([("readers", vec!["files"].into())]), "readers");
        assert!(allowed(&names, "files"));
        assert!(allowed(&names, "files#right"));
        assert!(!allowed(&names, "changes#right"));
        assert!(!allowed(&names, "editor/files#x"));
        assert!(!allowed(&[], "files"));
        assert!(super::names(&Data::Nil, "writers").is_empty());
    }

    #[test]
    fn status_entries_become_marks() {
        let bytes = b" M src/a.rs\0?? new.txt\0D  gone.rs\0UU both.rs\0A  added.rs\0MM twice.rs\0";
        let (changes, more) = status(bytes, 100);
        assert!(!more);
        let marks: Vec<(&str, char, bool)> = changes
            .iter()
            .map(|change| (change.path.as_str(), change.status, change.untracked))
            .collect();
        assert_eq!(
            marks,
            [
                ("src/a.rs", 'M', false),
                ("new.txt", 'A', true),
                ("gone.rs", 'D', false),
                ("both.rs", 'U', false),
                ("added.rs", 'A', false),
                ("twice.rs", 'M', false),
            ]
        );
        let (first, more) = status(bytes, 2);
        assert_eq!(first.len(), 2);
        assert!(more);
    }

    #[test]
    fn a_repositorys_filters_and_diff_programs_never_run() {
        let dir = tempdir();
        let marker = dir.join("ran");
        let git = |args: &[&str]| {
            assert!(
                new_command("git")
                    .arg("-C")
                    .arg(&dir)
                    .args(args)
                    .output()
                    .unwrap()
                    .status
                    .success()
            );
        };
        git(&["init", "-q"]);
        let touch = format!("touch '{}'; cat", marker.display());
        git(&["config", "filter.evil.clean", &touch]);
        git(&[
            "config",
            "filter.evil.process",
            &format!("touch '{}'", marker.display()),
        ]);
        git(&[
            "config",
            "diff.external",
            &format!("touch '{}'", marker.display()),
        ]);
        std::fs::write(dir.join(".gitattributes"), "*.txt filter=evil diff=evil\n").unwrap();
        git(&["config", "diff.evil.textconv", &touch]);
        std::fs::write(dir.join("a.txt"), "one\n").unwrap();
        // Staged by hand, with the filter emptied, so the look below is
        // the first thing that could run it.
        git(&[
            "-c",
            "filter.evil.clean=",
            "-c",
            "filter.evil.process=",
            "add",
            "-A",
        ]);
        std::fs::write(dir.join("a.txt"), "two\n").unwrap();
        let _ = std::fs::remove_file(&marker);
        read(
            &dir,
            &["status", "--porcelain=v1", "-z", "--untracked-files=all"],
            1 << 20,
        )
        .unwrap();
        read(
            &dir,
            &[
                "diff",
                "--cached",
                "--numstat",
                "-z",
                "--no-ext-diff",
                "--no-textconv",
            ],
            1 << 20,
        )
        .unwrap();
        read(
            &dir,
            &["diff", "--no-ext-diff", "--no-textconv", "--", "a.txt"],
            1 << 20,
        )
        .unwrap();
        assert!(!marker.exists(), "a repository's own program ran");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_filter_named_with_an_equals_sign_is_switched_off_too() {
        let dir = tempdir();
        let marker = dir.join("ran");
        let git = |args: &[&str]| {
            assert!(
                new_command("git")
                    .arg("-C")
                    .arg(&dir)
                    .args(args)
                    .output()
                    .unwrap()
                    .status
                    .success()
            );
        };
        git(&["init", "-q"]);
        // As `-c filter.a=b.clean=`, git would read the key `filter.a` and
        // leave this one on.
        git(&[
            "config",
            "filter.a=b.clean",
            &format!("touch '{}'; cat", marker.display()),
        ]);
        std::fs::write(dir.join(".gitattributes"), "*.txt filter=a=b\n").unwrap();
        std::fs::write(dir.join("a.txt"), "one\n").unwrap();
        git(&["-c", "filter.a=b.clean=cat", "add", "-A"]);
        std::fs::write(dir.join("a.txt"), "two\n").unwrap();
        let _ = std::fs::remove_file(&marker);
        // The filter is live: plain git runs it.
        git(&["status", "--porcelain"]);
        assert!(marker.exists(), "the test's filter never runs");
        std::fs::remove_file(&marker).unwrap();
        read(&dir, &["status", "--porcelain=v1", "-z"], 1 << 20).unwrap();
        assert!(!marker.exists(), "a repository's own filter ran");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_submodules_own_filters_never_run() {
        let dir = tempdir();
        let marker = dir.join("ran");
        let at = |path: &Path, args: &[&str]| {
            let output = new_command("git")
                .arg("-C")
                .arg(path)
                .args(args)
                .output()
                .unwrap();
            assert!(output.status.success(), "git {args:?}: {output:?}");
        };
        let inner = dir.join("inner");
        let outer = dir.join("outer");
        std::fs::create_dir_all(&inner).unwrap();
        std::fs::create_dir_all(&outer).unwrap();
        for repo in [&inner, &outer] {
            at(repo, &["init", "-q"]);
            at(repo, &["config", "user.name", "Test"]);
            at(repo, &["config", "user.email", "test@example.com"]);
            at(repo, &["config", "commit.gpgsign", "false"]);
        }
        std::fs::write(inner.join("a.txt"), "one\n").unwrap();
        std::fs::write(inner.join(".gitattributes"), "*.txt filter=sub\n").unwrap();
        at(&inner, &["add", "-A"]);
        at(&inner, &["commit", "-q", "-m", "inner"]);
        let source = inner.to_string_lossy().into_owned();
        at(
            &outer,
            &[
                "-c",
                "protocol.file.allow=always",
                "submodule",
                "add",
                "-q",
                &source,
                "sub",
            ],
        );
        at(&outer, &["commit", "-q", "-m", "outer"]);
        let sub = outer.join("sub");
        // The filter lives in the submodule's own config only.
        at(
            &sub,
            &[
                "config",
                "filter.sub.clean",
                &format!("touch '{}'; cat", marker.display()),
            ],
        );
        std::fs::write(sub.join("a.txt"), "two\n").unwrap();
        let _ = std::fs::remove_file(&marker);
        // The filter is live: plain git, from the outer repository, runs it.
        at(&outer, &["status", "--porcelain"]);
        assert!(marker.exists(), "the test's filter never runs");
        std::fs::remove_file(&marker).unwrap();
        read(
            &outer,
            &["status", "--porcelain=v1", "-z", "--untracked-files=all"],
            1 << 20,
        )
        .unwrap();
        read(
            &outer,
            &[
                "diff",
                "HEAD",
                "--numstat",
                "-z",
                "--no-ext-diff",
                "--no-textconv",
            ],
            1 << 20,
        )
        .unwrap();
        assert!(!marker.exists(), "a submodule's own filter ran");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn unreadable_config_refuses_the_read() {
        let dir = tempdir();
        let git = |args: &[&str]| {
            assert!(
                new_command("git")
                    .arg("-C")
                    .arg(&dir)
                    .args(args)
                    .output()
                    .unwrap()
                    .status
                    .success()
            );
        };
        git(&["init", "-q"]);
        std::fs::write(dir.join(".git/config"), "[core\nbroken").unwrap();
        assert!(read(&dir, &["status", "--porcelain=v1", "-z"], 1 << 20).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn git_reads_a_repository_and_bounds_its_output() {
        let dir = tempdir();
        let git = |args: &[&str]| {
            assert!(
                new_command("git")
                    .arg("-C")
                    .arg(&dir)
                    .args(args)
                    .output()
                    .unwrap()
                    .status
                    .success()
            );
        };
        git(&["init", "-q"]);
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        std::fs::write(dir.join("sub/a.txt"), "x".repeat(100)).unwrap();
        let top = toplevel(&dir.join("sub")).unwrap();
        assert_eq!(top.canonicalize().unwrap(), dir.canonicalize().unwrap());
        let sub = dir.join("sub").canonicalize().unwrap();
        assert_eq!(prefix(&top, &sub).unwrap(), "sub/");
        assert_eq!(prefix(&top, &top.canonicalize().unwrap()).unwrap(), "");
        let all = read(
            &dir,
            &["status", "--porcelain=v1", "-z", "--untracked-files=all"],
            1 << 20,
        )
        .unwrap();
        assert!(!all.truncated);
        assert_eq!(all.bytes, b"?? sub/a.txt\0");
        let cut = read(
            &dir,
            &["status", "--porcelain=v1", "-z", "--untracked-files=all"],
            4,
        )
        .unwrap();
        assert!(cut.truncated);
        assert_eq!(cut.bytes, b"?? s");
        assert!(read(&dir, &["no-such-command"], 100).is_err());
        let outside = tempdir();
        assert!(toplevel(&outside).is_none());
        std::fs::remove_dir_all(&dir).unwrap();
        std::fs::remove_dir_all(&outside).unwrap();
    }

    /// A fresh folder of the test's own under the system's temporary one.
    pub(crate) fn tempdir() -> PathBuf {
        use std::sync::atomic::{AtomicU32, Ordering};
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "herdr-moonframe-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }
}
