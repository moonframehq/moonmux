//! `app.folder`: the files of the folder the Files panel follows (the
//! focused pane's), listed lazily, and a bounded preview of one.
//!
//! Only the folder itself and the folders the panel has open are listed,
//! so a large tree costs what is shown. It looks again every few seconds.
//!
//! What it publishes:
//!
//! ```text
//! { available, folder,            -- the folder as the mod named it
//!   name,                         -- its last part
//!   dirs = { [""] = { { name, dir } }, ["src"] = ... },  -- folders first
//!   marks = { ["src/a.rs"] = "M" },  -- git's changes, A M D U
//!   limited }                     -- some folder had more than shown
//! ```
//!
//! What it takes: `{ root = "<folder>", open = { "src", "src/app" } }` (the
//! folder, and its open folders, the whole set each time), and
//! `{ read = "<path>", id }`, answered to the asking mod as
//! `{ reply = id, action = "read", ok, path, text, truncated | error }`.
//!
//! A mod may name any folder on the machine, so only the mods in
//! provider.toml's `[settings] readers` are listened to.

use {
    crate::git,
    moonframe::{Data, Handle, Native},
    serde_json::{Map, Value, json},
    std::{
        collections::BTreeMap,
        fs,
        io::Read,
        path::{Path, PathBuf},
    },
};

/// Entries listed per folder.
const PER_FOLDER: usize = 500;
/// Entries listed in all.
const TOTAL: usize = 2000;
/// Folders listed besides the root.
const MAX_OPEN: usize = 100;
/// Ticks (seconds) between looks.
const LOOK_EVERY: u32 = 3;
/// Bytes of a file a preview reads.
const PREVIEW_BYTES: usize = 256 * 1024;
/// Lines a preview shows, each cut at `LINE_CHARS`: the view draws each.
const PREVIEW_LINES: usize = 1500;
const LINE_CHARS: usize = 400;
/// Bytes of `git status` read for the marks.
const STATUS_BYTES: usize = 1 << 20;
/// Never listed, as an editor hides them.
const HIDDEN: &[&str] = &[".DS_Store", ".git"];

#[derive(Default)]
pub(crate) struct Folder {
    /// The mods listened to (`[settings] readers`).
    readers: Vec<String>,
    /// The folder as the mod named it, and where it is.
    asked: String,
    root: Option<PathBuf>,
    /// Open folders, from the root.
    open: Vec<String>,
    ticks: u32,
    /// What was last published, so an unchanged look redraws nothing.
    last: Value,
}

impl Folder {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    fn snapshot(&self) -> Value {
        if self.asked.is_empty() {
            return json!({ "available": false, "reason": "No folder" });
        }
        let Some(root) = self.root.as_deref().filter(|root| root.is_dir()) else {
            return json!({ "available": false, "folder": self.asked, "reason": "This folder is not there" });
        };
        let (dirs, limited) = list(root, &self.open);
        json!({
            "available": true,
            "folder": self.asked,
            "name": root.file_name().map(|name| name.to_string_lossy()).unwrap_or_default(),
            "dirs": dirs,
            "marks": marks(root),
            "limited": limited,
        })
    }

    fn publish(&mut self, host: &Handle) {
        let snapshot = self.snapshot();
        if snapshot != self.last {
            host.publish_json(&snapshot);
            self.last = snapshot;
        }
    }

    fn follow(&mut self, asked: &str, open: &[&str]) {
        if asked != self.asked {
            self.asked = asked.to_owned();
            self.root = git::folder(asked);
        }
        self.open = open
            .iter()
            .filter(|path| git::plain(path))
            .take(MAX_OPEN)
            .map(|path| (*path).to_owned())
            .collect();
    }
}

impl Native for Folder {
    fn start(&mut self, host: &Handle) -> Result<(), String> {
        self.readers = git::names(host.settings(), "readers");
        self.publish(host);
        Ok(())
    }

    fn tick(&mut self, host: &Handle) {
        self.ticks += 1;
        if self.ticks >= LOOK_EVERY {
            self.ticks = 0;
            self.publish(host);
        }
    }

    fn receive(&mut self, host: &Handle, from: &str, message: Data) {
        let Data::Map(fields) = &message else {
            return;
        };
        let text = |key: &str| match fields.get(key) {
            Some(Data::String(text)) => Some(text.as_str()),
            _ => None,
        };
        let allowed = git::allowed(&self.readers, from);
        if let Some(path) = text("read") {
            let reply = fields.get("id").cloned().unwrap_or_else(|| "read".into());
            let answer = if !allowed {
                Err(format!(
                    "`{from}` may not read files: only the mods in providers/folder's `readers` may"
                ))
            } else {
                match &self.root {
                    Some(root) => preview(root, path),
                    None => Err("No folder".to_owned()),
                }
            };
            host.send(from, read_reply(reply, path, answer));
            return;
        }
        if !allowed {
            return;
        }
        if let Some(asked) = text("root") {
            // A list, not lines: a folder's name may hold a newline.
            let open: Vec<&str> = match fields.get("open") {
                Some(Data::List(paths)) => paths
                    .iter()
                    .filter_map(|path| match path {
                        Data::String(path) => Some(path.as_str()),
                        _ => None,
                    })
                    .collect(),
                _ => Vec::new(),
            };
            self.follow(asked, &open);
            self.ticks = 0;
            self.publish(host);
        }
    }
}

/// What a `read` answers.
fn read_reply(reply: Data, path: &str, answer: Result<(String, bool), String>) -> Data {
    let mut fields = BTreeMap::from([
        ("reply".to_owned(), reply),
        ("action".to_owned(), Data::from("read")),
        ("path".to_owned(), Data::from(path)),
        ("ok".to_owned(), Data::Bool(answer.is_ok())),
    ]);
    match answer {
        Ok((text, truncated)) => {
            fields.insert("text".to_owned(), Data::String(text));
            fields.insert("truncated".to_owned(), Data::Bool(truncated));
        }
        Err(error) => {
            fields.insert("error".to_owned(), Data::String(error));
        }
    }
    Data::Map(fields)
}

/// The root's entries and each open folder's, folders first; whether any
/// was cut short.
fn list(root: &Path, open: &[String]) -> (Value, bool) {
    let mut dirs = Map::new();
    let mut total = 0;
    let mut limited = false;
    let mut folders = vec![String::new()];
    folders.extend(open.iter().cloned());
    for folder in folders {
        let path = if folder.is_empty() {
            Some(root.to_owned())
        } else {
            git::inside(root, &folder).filter(|path| path.is_dir())
        };
        let Some(path) = path else { continue };
        let (entries, cut) = entries(&path, PER_FOLDER.min(TOTAL - total));
        total += entries.len();
        limited |= cut;
        dirs.insert(folder, Value::Array(entries));
        if total >= TOTAL {
            limited = true;
            break;
        }
    }
    (Value::Object(dirs), limited)
}

/// At most `max` entries of `dir`, folders first, each group sorted
/// without regard to case; whether there were more.
fn entries(dir: &Path, max: usize) -> (Vec<Value>, bool) {
    let Ok(read) = fs::read_dir(dir) else {
        return (Vec::new(), false);
    };
    let mut found: Vec<(bool, String)> = Vec::new();
    let mut more = false;
    for entry in read.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if HIDDEN.contains(&name.as_str()) {
            continue;
        }
        if found.len() == max {
            more = true;
            break;
        }
        // A link to a folder opens as one.
        let is_dir = fs::metadata(entry.path()).is_ok_and(|meta| meta.is_dir());
        found.push((is_dir, name));
    }
    found.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then_with(|| a.1.to_lowercase().cmp(&b.1.to_lowercase()))
            .then_with(|| a.1.cmp(&b.1))
    });
    let entries = found
        .into_iter()
        .map(|(dir, name)| json!({ "name": name, "dir": dir }))
        .collect();
    (entries, more)
}

/// git's changes under `root`, by path from it.
fn marks(root: &Path) -> Value {
    let mut marks = Map::new();
    let Some(top) = git::toplevel(root) else {
        return Value::Object(marks);
    };
    let Some(prefix) = git::prefix(&top, root) else {
        return Value::Object(marks);
    };
    let args = [
        "status",
        "--porcelain=v1",
        "-z",
        "--no-renames",
        "--untracked-files=all",
    ];
    let Ok(output) = git::read(&top, &args, STATUS_BYTES) else {
        return Value::Object(marks);
    };
    let (changes, _) = git::status(&output.bytes, TOTAL);
    for change in changes {
        if let Some(path) = change.path.strip_prefix(&prefix) {
            marks.insert(path.to_owned(), Value::String(change.status.to_string()));
        }
    }
    Value::Object(marks)
}

/// The start of the text of `relative` under `root`, and whether it was
/// cut; a binary file or one outside the root is refused.
fn preview(root: &Path, relative: &str) -> Result<(String, bool), String> {
    let path = git::inside(root, relative).ok_or("This file is not in the folder")?;
    if !path.is_file() {
        return Err("This file is no longer there".to_owned());
    }
    let file = fs::File::open(&path).map_err(|_| "Cannot read this file".to_owned())?;
    let mut bytes = Vec::new();
    file.take(PREVIEW_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Cannot read this file".to_owned())?;
    if bytes.contains(&0) {
        return Err("Binary file · no text preview".to_owned());
    }
    let mut truncated = bytes.len() > PREVIEW_BYTES;
    bytes.truncate(PREVIEW_BYTES);
    let text = String::from_utf8_lossy(&bytes);
    let mut lines = Vec::new();
    for (index, line) in text.lines().enumerate() {
        if index == PREVIEW_LINES {
            truncated = true;
            break;
        }
        lines.push(cut(line, LINE_CHARS));
    }
    Ok((lines.join("\n"), truncated))
}

/// `line` cut at `max` characters, with an ellipsis when it was longer.
pub(crate) fn cut(line: &str, max: usize) -> String {
    match line.char_indices().nth(max) {
        Some((at, _)) => format!("{}…", &line[..at]),
        None => line.to_owned(),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use {super::*, crate::git::tests::tempdir, std::sync::Arc};

    fn project() -> PathBuf {
        let dir = tempdir();
        fs::create_dir_all(dir.join("src/app")).unwrap();
        fs::create_dir_all(dir.join(".git")).unwrap();
        fs::write(dir.join("README.md"), "# hi\n").unwrap();
        fs::write(dir.join("b.txt"), "b").unwrap();
        fs::write(dir.join(".DS_Store"), "x").unwrap();
        fs::write(dir.join("src/main.rs"), "fn main() {}\n").unwrap();
        fs::write(dir.join("src/app/x.luau"), "return 1\n").unwrap();
        dir.canonicalize().unwrap()
    }

    #[test]
    fn folders_come_first_and_hidden_entries_never_show() {
        let root = project();
        let (dirs, limited) = list(&root, &["src".into(), "../etc".into(), "nope".into()]);
        assert!(!limited);
        let names = |key: &str| -> Vec<(String, bool)> {
            dirs[key]
                .as_array()
                .unwrap()
                .iter()
                .map(|entry| {
                    (
                        entry["name"].as_str().unwrap().to_owned(),
                        entry["dir"].as_bool().unwrap(),
                    )
                })
                .collect()
        };
        assert_eq!(
            names(""),
            [
                ("src".into(), true),
                ("b.txt".into(), false),
                ("README.md".into(), false)
            ]
        );
        assert_eq!(
            names("src"),
            [("app".into(), true), ("main.rs".into(), false)]
        );
        // Only plain folders inside the root are listed.
        assert!(dirs.get("../etc").is_none());
        assert!(dirs.get("nope").is_none());
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_long_folder_is_cut_and_says_so() {
        let root = tempdir();
        for index in 0..5 {
            fs::write(root.join(format!("f{index}")), "").unwrap();
        }
        let (entries, more) = entries(&root, 3);
        assert_eq!(entries.len(), 3);
        assert!(more);
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn previews_are_bounded_text_inside_the_folder() {
        let root = project();
        assert_eq!(
            preview(&root, "src/main.rs").unwrap(),
            ("fn main() {}".into(), false)
        );
        fs::write(root.join("bin"), [0u8, 1, 2]).unwrap();
        assert!(preview(&root, "bin").unwrap_err().contains("Binary"));
        assert!(preview(&root, "../x").is_err());
        assert!(preview(&root, "src").is_err());
        let long = "x\n".repeat(PREVIEW_LINES + 10);
        fs::write(root.join("long.txt"), long).unwrap();
        let (text, truncated) = preview(&root, "long.txt").unwrap();
        assert!(truncated);
        assert_eq!(text.lines().count(), PREVIEW_LINES);
        fs::write(root.join("wide.txt"), "é".repeat(LINE_CHARS + 5)).unwrap();
        let (text, _) = preview(&root, "wide.txt").unwrap();
        assert_eq!(text.chars().count(), LINE_CHARS + 1);
        assert!(text.ends_with('…'));
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn only_readers_are_listened_to() {
        let root = project();
        let settings = Data::map([("readers", vec!["files"].into())]);
        let host = Handle::new("folder", &root, settings, Arc::new(|_| {}));
        let mut folder = Folder::new();
        folder.start(&host).unwrap();
        let asked = root.to_string_lossy().into_owned();
        let follow = Data::map([
            ("root", Data::from(asked.as_str())),
            ("open", Data::List(vec!["src".into()])),
        ]);
        folder.receive(&host, "someones-package", follow.clone());
        assert!(folder.root.is_none());
        folder.receive(&host, "files#right", follow);
        assert_eq!(folder.root.as_deref(), Some(root.as_path()));
        assert_eq!(folder.open, ["src"]);
        let snapshot = folder.snapshot();
        assert_eq!(snapshot["available"], true);
        assert_eq!(snapshot["folder"], asked.as_str());
        assert!(snapshot["dirs"]["src"].is_array());
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn a_folder_named_with_a_newline_opens() {
        let root = project();
        fs::create_dir_all(root.join("a\nb")).unwrap();
        fs::write(root.join("a\nb/inside.txt"), "").unwrap();
        let settings = Data::map([("readers", vec!["files"].into())]);
        let host = Handle::new("folder", &root, settings, Arc::new(|_| {}));
        let mut folder = Folder::new();
        folder.start(&host).unwrap();
        let asked = root.to_string_lossy().into_owned();
        let open = Data::List(vec!["a\nb".into()]);
        folder.receive(
            &host,
            "files",
            Data::map([("root", Data::from(asked.as_str())), ("open", open)]),
        );
        assert_eq!(folder.open, ["a\nb"]);
        assert_eq!(folder.snapshot()["dirs"]["a\nb"][0]["name"], "inside.txt");
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_missing_folder_says_so() {
        let mut folder = Folder::new();
        assert_eq!(folder.snapshot()["reason"], "No folder");
        folder.follow("/no/such/folder/here", &[]);
        assert_eq!(folder.snapshot()["available"], false);
        folder.follow("relative/path", &[]);
        assert!(folder.root.is_none());
    }

    #[test]
    fn marks_are_relative_to_the_folder() {
        let root = tempdir();
        let ok = |args: &[&str]| {
            assert!(
                crate::core::new_command("git")
                    .arg("-C")
                    .arg(&root)
                    .args(args)
                    .output()
                    .unwrap()
                    .status
                    .success()
            )
        };
        ok(&["init", "-q"]);
        fs::create_dir_all(root.join("sub")).unwrap();
        fs::write(root.join("sub/new.txt"), "n").unwrap();
        fs::write(root.join("top.txt"), "t").unwrap();
        let sub = root.join("sub").canonicalize().unwrap();
        let marks = marks(&sub);
        assert_eq!(marks, json!({ "new.txt": "A" }));
        fs::remove_dir_all(&root).unwrap();
    }
}
