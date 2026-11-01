//! `app.changes`: the uncommitted changes of the repository holding the
//! folder the Changes panel follows (the focused pane's), one file's diff,
//! and a commit of them all.
//!
//! It looks every two seconds and publishes only when `git status` or the
//! line counts changed.
//!
//! What it publishes:
//!
//! ```text
//! { available, folder,              -- the folder as the mod named it
//!   repository,                     -- false outside one
//!   root, name, branch,
//!   files = { { path, status, added, deleted } },  -- at most 300
//!   changed, added, deleted, truncated,
//!   diff = { path, binary, truncated,
//!            hunks = { { header, lines = { { kind, old?, new?, text } } } } } }
//! ```
//!
//! What it takes: `{ root = "<folder>" }`, `{ diff = "<path>" | false }`,
//! and `{ commit = "<message>", id }` (stages everything and commits),
//! answered to the asking mod as `{ reply = id, action = "commit", ok,
//! message | error }`.
//!
//! A commit changes the user's history, and a mod may name any folder, so
//! only the mods in provider.toml's `[settings] writers` are listened to.

use {
    crate::{core::new_command, folder::cut, git},
    moonframe::{Data, Handle, Native},
    serde_json::{Value, json},
    std::{
        collections::{BTreeMap, HashMap},
        fs,
        io::Read,
        path::{Path, PathBuf},
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
        thread,
    },
};

/// Files listed.
const MAX_FILES: usize = 300;
/// Diff lines shown, each cut at `LINE_CHARS`: the view draws each.
const DIFF_LINES: usize = 1000;
const LINE_CHARS: usize = 400;
/// Bytes read of `git status`, of the numstat, and of one file's diff.
const STATUS_BYTES: usize = 1 << 20;
const DIFF_BYTES: usize = 512 * 1024;
/// Bytes of an untracked file read to count its lines or show it.
const UNTRACKED_BYTES: usize = 256 * 1024;
/// Ticks (seconds) between looks.
const LOOK_EVERY: u32 = 2;
/// git's empty tree: the base before a first commit.
const EMPTY_TREE: &str = "4b825dc642cb6eb9a060e54bf8d69288fbee4904";

/// One changed file.
#[derive(Clone, Debug, PartialEq)]
struct File {
    change: git::Change,
    added: u64,
    deleted: u64,
    binary: bool,
}

#[derive(Default)]
pub(crate) struct Changes {
    /// The mods listened to (`[settings] writers`).
    writers: Vec<String>,
    /// The folder as the mod named it, and its repository's top.
    asked: String,
    root: Option<PathBuf>,
    /// The file whose diff is published.
    diff_path: Option<String>,
    /// Status and numstat last read: a look finding them the same and the
    /// same file asked for publishes nothing.
    last_look: Vec<u8>,
    ticks: u32,
    last: Value,
    /// A commit runs: one at a time.
    busy: Arc<AtomicBool>,
    /// A commit finished: look again at the next tick.
    stale: Arc<AtomicBool>,
}

impl Changes {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Looks at the repository; publishes what changed, or everything when
    /// `force`.
    fn refresh(&mut self, host: &Handle, force: bool) {
        let snapshot = self.snapshot(force);
        if let Some(snapshot) = snapshot
            && (force || snapshot != self.last)
        {
            host.publish_json(&snapshot);
            self.last = snapshot;
        }
    }

    /// What to publish, or nothing when the look found what it last did.
    fn snapshot(&mut self, force: bool) -> Option<Value> {
        if self.asked.is_empty() {
            return Some(json!({ "available": false, "reason": "No folder" }));
        }
        let Some(root) = self.root.clone() else {
            return Some(json!({ "available": true, "folder": self.asked, "repository": false }));
        };
        let status_args = [
            "status",
            "--porcelain=v1",
            "-z",
            "--no-renames",
            "--untracked-files=all",
        ];
        let Ok(status) = git::read(&root, &status_args, STATUS_BYTES) else {
            return Some(json!({ "available": true, "folder": self.asked, "repository": false }));
        };
        let base = base(&root);
        let numstat = git::read(
            &root,
            &[
                "diff",
                &base,
                "--numstat",
                "-z",
                "--no-renames",
                "--no-ext-diff",
                "--no-textconv",
            ],
            STATUS_BYTES,
        )
        .map(|output| output.bytes)
        .unwrap_or_default();
        let mut look = status.bytes.clone();
        look.push(0xff);
        look.extend_from_slice(&numstat);
        look.extend_from_slice(self.diff_path.as_deref().unwrap_or("").as_bytes());
        if !force && look == self.last_look {
            return None;
        }
        self.last_look = look;
        let (changes, more) = git::status(&status.bytes, MAX_FILES);
        let files = files(&root, changes, &numstat);
        let branch = git::read(&root, &["rev-parse", "--abbrev-ref", "HEAD"], 4096)
            .map(|output| String::from_utf8_lossy(&output.bytes).trim().to_owned())
            .unwrap_or_default();
        let diff = self
            .diff_path
            .as_deref()
            .and_then(|path| files.iter().find(|file| file.change.path == path))
            .map(|file| diff(&root, &base, file));
        let (added, deleted) = files
            .iter()
            .fold((0, 0), |(a, d), file| (a + file.added, d + file.deleted));
        Some(json!({
            "available": true,
            "folder": self.asked,
            "repository": true,
            "root": root.to_string_lossy(),
            "name": root.file_name().map(|name| name.to_string_lossy()).unwrap_or_default(),
            "branch": branch,
            "files": files.iter().map(|file| json!({
                "path": file.change.path,
                "status": file.change.status.to_string(),
                "added": file.added,
                "deleted": file.deleted,
            })).collect::<Vec<_>>(),
            "changed": files.len(),
            "added": added,
            "deleted": deleted,
            "truncated": more || status.truncated,
            "diff": diff,
        }))
    }

    fn commit(&mut self, host: &Handle, from: &str, reply: Data, message: &str) {
        let answer = move |result: Result<String, String>| {
            let (key, text) = match result {
                Ok(text) => ("message", text),
                Err(text) => ("error", text),
            };
            Data::Map(BTreeMap::from([
                ("reply".to_owned(), reply.clone()),
                ("action".to_owned(), Data::from("commit")),
                ("ok".to_owned(), Data::Bool(key == "message")),
                (key.to_owned(), Data::String(text)),
            ]))
        };
        let message = message.trim().to_owned();
        let refused = if !git::allowed(&self.writers, from) {
            Some(format!(
                "`{from}` may not commit: only the mods in providers/changes's `writers` may"
            ))
        } else if message.is_empty() {
            Some("Write a commit message first".to_owned())
        } else if self.root.is_none() {
            Some("Not a git repository".to_owned())
        } else if self.busy.swap(true, Ordering::AcqRel) {
            Some("A commit is already running".to_owned())
        } else {
            None
        };
        if let Some(error) = refused {
            host.send(from, answer(Err(error)));
            return;
        }
        let Some(root) = self.root.clone() else {
            return;
        };
        let (busy, stale) = (self.busy.clone(), self.stale.clone());
        let (host, from) = (host.clone(), from.to_owned());
        // Signing may wait on the user's hardware key: never on the
        // provider's thread.
        let spawned = thread::Builder::new()
            .name("changes-commit".into())
            .spawn(move || {
                host.send(&from, answer(commit(&root, &message)));
                busy.store(false, Ordering::Release);
                stale.store(true, Ordering::Release);
            });
        if spawned.is_err() {
            self.busy.store(false, Ordering::Release);
        }
    }
}

impl Native for Changes {
    fn start(&mut self, host: &Handle) -> Result<(), String> {
        self.writers = git::names(host.settings(), "writers");
        self.refresh(host, true);
        Ok(())
    }

    fn tick(&mut self, host: &Handle) {
        self.ticks += 1;
        let stale = self.stale.swap(false, Ordering::AcqRel);
        if stale || self.ticks >= LOOK_EVERY {
            self.ticks = 0;
            self.refresh(host, stale);
        }
    }

    fn receive(&mut self, host: &Handle, from: &str, message: Data) {
        let Data::Map(fields) = &message else {
            return;
        };
        if let Some(Data::String(text)) = fields.get("commit") {
            let reply = fields.get("id").cloned().unwrap_or_else(|| "commit".into());
            self.commit(host, from, reply, text);
            return;
        }
        if !git::allowed(&self.writers, from) {
            return;
        }
        if let Some(Data::String(asked)) = fields.get("root")
            && *asked != self.asked
        {
            self.asked = asked.clone();
            self.root = git::folder(asked).and_then(|folder| git::toplevel(&folder));
            self.diff_path = None;
            // Another repository as clean as the last still says which.
            self.last_look.clear();
        }
        match fields.get("diff") {
            Some(Data::String(path)) => self.diff_path = Some(path.clone()),
            Some(Data::Bool(false)) => self.diff_path = None,
            _ => {}
        }
        self.ticks = 0;
        self.refresh(host, false);
    }
}

/// HEAD, or the empty tree before a first commit.
fn base(root: &Path) -> String {
    if git::read(root, &["rev-parse", "-q", "--verify", "HEAD"], 4096).is_ok() {
        "HEAD".to_owned()
    } else {
        EMPTY_TREE.to_owned()
    }
}

/// The changes with their line counts: the numstat's, or an untracked
/// file's own lines.
fn files(root: &Path, changes: Vec<git::Change>, numstat: &[u8]) -> Vec<File> {
    let counts = counts(numstat);
    changes
        .into_iter()
        .map(|change| {
            let (added, deleted, binary) = if change.untracked {
                match untracked(root, &change.path) {
                    Some(text) => (lines(&text) as u64, 0, false),
                    None => (0, 0, true),
                }
            } else {
                counts.get(&change.path).copied().unwrap_or((0, 0, false))
            };
            File {
                change,
                added,
                deleted,
                binary,
            }
        })
        .collect()
}

/// `git diff --numstat -z --no-renames`: lines added and deleted by path,
/// and whether the file is binary.
fn counts(numstat: &[u8]) -> HashMap<String, (u64, u64, bool)> {
    let mut counts = HashMap::new();
    for entry in numstat.split(|byte| *byte == 0) {
        let text = String::from_utf8_lossy(entry);
        let mut parts = text.splitn(3, '\t');
        let (Some(added), Some(deleted), Some(path)) = (parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        let binary = added == "-";
        counts.insert(
            path.to_owned(),
            (
                added.parse().unwrap_or(0),
                deleted.parse().unwrap_or(0),
                binary,
            ),
        );
    }
    counts
}

/// The text of an untracked file, bounded; nothing for a binary or
/// unreadable one.
fn untracked(root: &Path, path: &str) -> Option<String> {
    let full = git::inside(&root.canonicalize().ok()?, path)?;
    let mut bytes = Vec::new();
    fs::File::open(full)
        .ok()?
        .take(UNTRACKED_BYTES as u64)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.contains(&0) {
        return None;
    }
    Some(String::from_utf8_lossy(&bytes).into_owned())
}

fn lines(text: &str) -> usize {
    text.lines().count()
}

/// One file's diff against `base`, as `diff-view` draws it.
fn diff(root: &Path, base: &str, file: &File) -> Value {
    let path = &file.change.path;
    if file.change.untracked {
        let Some(text) = untracked(root, path) else {
            return json!({ "path": path, "binary": true, "truncated": false, "hunks": [] });
        };
        let all: Vec<&str> = text.lines().collect();
        let shown: Vec<Value> = all
            .iter()
            .take(DIFF_LINES)
            .enumerate()
            .map(|(index, line)| json!({ "kind": "add", "new": index + 1, "text": cut(line, LINE_CHARS) }))
            .collect();
        let header = format!("@@ -0,0 +1,{} @@", all.len());
        return json!({
            "path": path,
            "binary": false,
            "truncated": all.len() > DIFF_LINES,
            "hunks": [{ "header": header, "lines": shown }],
        });
    }
    let args = [
        "diff",
        base,
        "--no-ext-diff",
        "--no-textconv",
        "--no-color",
        "--",
        path.as_str(),
    ];
    let Ok(output) = git::read(root, &args, DIFF_BYTES) else {
        return json!({ "path": path, "binary": file.binary, "truncated": false, "hunks": [] });
    };
    let (hunks, cut_short) = hunks(&String::from_utf8_lossy(&output.bytes));
    json!({
        "path": path,
        "binary": file.binary,
        "truncated": cut_short || output.truncated,
        "hunks": hunks,
    })
}

/// The hunks of a unified diff, at most `DIFF_LINES` lines in all; whether
/// it was cut short.
fn hunks(text: &str) -> (Vec<Value>, bool) {
    let mut hunks: Vec<Value> = Vec::new();
    let mut lines: Vec<Value> = Vec::new();
    let mut header = String::new();
    let (mut old, mut new) = (0u64, 0u64);
    let mut count = 0;
    let mut truncated = false;
    let close = |hunks: &mut Vec<Value>, header: &str, lines: &mut Vec<Value>| {
        if !header.is_empty() {
            hunks.push(json!({ "header": header, "lines": std::mem::take(lines) }));
        }
    };
    for line in text.lines() {
        if let Some((from, to)) = starts(line) {
            close(&mut hunks, &header, &mut lines);
            header = cut(line, LINE_CHARS);
            (old, new) = (from, to);
            continue;
        }
        if header.is_empty() || line.starts_with('\\') {
            continue;
        }
        if count == DIFF_LINES {
            truncated = true;
            break;
        }
        count += 1;
        let (kind, rest) = line.split_at(line.chars().next().map_or(0, char::len_utf8));
        let text = cut(rest, LINE_CHARS);
        let entry = match kind {
            "+" => {
                new += 1;
                json!({ "kind": "add", "new": new - 1, "text": text })
            }
            "-" => {
                old += 1;
                json!({ "kind": "del", "old": old - 1, "text": text })
            }
            _ => {
                old += 1;
                new += 1;
                json!({ "kind": "ctx", "old": old - 1, "new": new - 1, "text": text })
            }
        };
        lines.push(entry);
    }
    close(&mut hunks, &header, &mut lines);
    (hunks, truncated)
}

/// The old and new start lines of a `@@ -a,b +c,d @@` header.
fn starts(line: &str) -> Option<(u64, u64)> {
    let rest = line.strip_prefix("@@ -")?;
    let (old, rest) = rest.split_once(" +")?;
    let (new, _) = rest.split_once(" @@")?;
    let first = |range: &str| range.split(',').next()?.parse().ok();
    Some((first(old)?, first(new)?))
}

/// Stages everything in `root` and commits it with `message`.
fn commit(root: &Path, message: &str) -> Result<String, String> {
    let run = |args: &[&str]| -> Result<String, String> {
        let output = new_command("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .output()
            .map_err(|error| format!("git {}: {error}", args[0]))?;
        let say = |bytes: &[u8]| String::from_utf8_lossy(bytes).trim().to_owned();
        if output.status.success() {
            Ok(say(&output.stdout))
        } else {
            let said = say(&output.stderr);
            Err(if said.is_empty() {
                say(&output.stdout)
            } else {
                said
            })
        }
    };
    run(&["add", "-A"])?;
    let said = run(&["commit", "-q", "-m", message])?;
    Ok(if said.is_empty() {
        "Committed".to_owned()
    } else {
        said
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests;
