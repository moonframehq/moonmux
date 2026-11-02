use {super::*, crate::git::tests::tempdir, std::sync::Mutex};

fn git(dir: &Path, args: &[&str]) {
    let output = new_command("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .unwrap();
    assert!(output.status.success(), "git {args:?}: {output:?}");
}

/// A repository with one commit of `a.txt`, committing as a test user
/// and never signing.
fn repository() -> PathBuf {
    let dir = tempdir();
    git(&dir, &["init", "-q"]);
    for (key, value) in [
        ("user.name", "Test"),
        ("user.email", "test@example.com"),
        ("commit.gpgsign", "false"),
        ("core.hooksPath", "/dev/null"),
    ] {
        git(&dir, &["config", key, value]);
    }
    fs::write(dir.join("a.txt"), "one\ntwo\n").unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "first"]);
    dir
}

fn writers() -> Data {
    Data::map([("writers", vec!["changes"].into())])
}

#[test]
fn hunks_number_their_lines() {
    let text = "diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -1,3 +1,3 @@ fn\n one\n-two\n+deux\n three\n\\ No newline at end of file\n@@ -10 +10,2 @@\n ten\n+eleven\n";
    let (hunks, truncated) = hunks(text);
    assert!(!truncated);
    assert_eq!(hunks.len(), 2);
    assert_eq!(hunks[0]["header"], "@@ -1,3 +1,3 @@ fn");
    assert_eq!(
        hunks[0]["lines"],
        json!([
            { "kind": "ctx", "old": 1, "new": 1, "text": "one" },
            { "kind": "del", "old": 2, "text": "two" },
            { "kind": "add", "new": 2, "text": "deux" },
            { "kind": "ctx", "old": 3, "new": 3, "text": "three" },
        ])
    );
    assert_eq!(
        hunks[1]["lines"][1],
        json!({ "kind": "add", "new": 11, "text": "eleven" })
    );
}

#[test]
fn a_long_diff_is_cut() {
    let mut text = String::from("@@ -1 +1,2000 @@\n");
    for index in 0..2000 {
        text.push_str(&format!("+{index}\n"));
    }
    let (hunks, truncated) = hunks(&text);
    assert!(truncated);
    assert_eq!(hunks[0]["lines"].as_array().unwrap().len(), DIFF_LINES);
}

#[test]
fn hunk_headers_give_their_starts() {
    assert_eq!(starts("@@ -1,3 +4,5 @@"), Some((1, 4)));
    assert_eq!(starts("@@ -7 +7 @@ fn main()"), Some((7, 7)));
    assert_eq!(starts("@@ bad"), None);
    assert_eq!(starts("+@@ -1 +1 @@"), None);
}

#[test]
fn numstat_counts_lines_and_spots_binaries() {
    let counts = counts(b"3\t1\tsrc/a.rs\0-\t-\timage.png\0\0");
    assert_eq!(counts["src/a.rs"], (3, 1, false));
    assert_eq!(counts["image.png"], (0, 0, true));
    assert_eq!(counts.len(), 2);
}

#[test]
fn a_repository_is_published_with_a_diff_and_commits_for_its_writer() {
    let dir = repository();
    fs::write(dir.join("a.txt"), "one\n2\n").unwrap();
    fs::write(dir.join("b.txt"), "new\nfile\n").unwrap();
    let sent = Arc::new(Mutex::new(0));
    let counter = sent.clone();
    let host = Handle::new(
        "changes",
        &dir,
        writers(),
        Arc::new(move |_| *counter.lock().unwrap() += 1),
    );
    let mut changes = Changes::new();
    changes.start(&host).unwrap();
    assert_eq!(changes.snapshot(true).unwrap()["reason"], "No folder");

    let asked = dir.to_string_lossy().into_owned();
    // Another mod is not listened to.
    changes.receive(
        &host,
        "someones-package",
        Data::map([("root", Data::from(asked.as_str()))]),
    );
    assert!(changes.root.is_none());
    changes.receive(
        &host,
        "changes#right",
        Data::map([
            ("root", Data::from(asked.as_str())),
            ("diff", "a.txt".into()),
        ]),
    );
    let snapshot = changes.snapshot(true).unwrap();
    assert_eq!(snapshot["repository"], true);
    assert_eq!(snapshot["folder"], asked.as_str());
    assert_eq!(snapshot["changed"], 2);
    assert_eq!(snapshot["added"], 3);
    assert_eq!(snapshot["deleted"], 1);
    let files = snapshot["files"].as_array().unwrap();
    assert!(files.contains(&json!({ "path": "a.txt", "status": "M", "added": 1, "deleted": 1 })));
    assert!(files.contains(&json!({ "path": "b.txt", "status": "A", "added": 2, "deleted": 0 })));
    assert_eq!(snapshot["diff"]["path"], "a.txt");
    let lines = &snapshot["diff"]["hunks"][0]["lines"];
    assert!(
        lines
            .as_array()
            .unwrap()
            .contains(&json!({ "kind": "add", "new": 2, "text": "2" }))
    );
    // Unchanged, a look publishes nothing.
    assert!(changes.snapshot(false).is_none());

    // An untracked file's diff is its lines.
    changes.receive(&host, "changes", Data::map([("diff", "b.txt".into())]));
    let snapshot = changes.snapshot(true).unwrap();
    assert_eq!(snapshot["diff"]["hunks"][0]["header"], "@@ -0,0 +1,2 @@");

    // An empty message and another mod are refused; the writer commits.
    changes.commit(&host, "changes", "c".into(), "   ");
    changes.commit(&host, "someones-package", "c".into(), "nope");
    changes.commit(&host, "changes#right", "c".into(), "second");
    for _ in 0..100 {
        if !changes.busy.load(Ordering::Acquire) {
            break;
        }
        thread::sleep(std::time::Duration::from_millis(50));
    }
    let snapshot = changes.snapshot(true).unwrap();
    assert_eq!(snapshot["changed"], 0, "{snapshot}");
    assert!(*sent.lock().unwrap() > 0);
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn outside_a_repository_it_says_so() {
    let dir = tempdir();
    let host = Handle::new("changes", &dir, writers(), Arc::new(|_| {}));
    let mut changes = Changes::new();
    changes.start(&host).unwrap();
    let asked = dir.to_string_lossy().into_owned();
    changes.receive(
        &host,
        "changes",
        Data::map([("root", Data::from(asked.as_str()))]),
    );
    let snapshot = changes.snapshot(true).unwrap();
    assert_eq!(snapshot["available"], true);
    assert_eq!(snapshot["repository"], false);
    assert_eq!(snapshot["folder"], asked.as_str());
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn before_a_first_commit_the_base_is_the_empty_tree() {
    let dir = tempdir();
    git(&dir, &["init", "-q"]);
    assert_eq!(base(&dir), EMPTY_TREE);
    fs::remove_dir_all(&dir).unwrap();
}

/// A `changes` provider following `dir`, for the mod `changes`.
fn following(dir: &Path) -> (Changes, Handle) {
    let host = Handle::new("changes", dir, writers(), Arc::new(|_| {}));
    let mut changes = Changes::new();
    changes.start(&host).unwrap();
    let asked = dir.to_string_lossy().into_owned();
    changes.receive(
        &host,
        "changes",
        Data::map([("root", Data::from(asked.as_str()))]),
    );
    (changes, host)
}

/// Sets `path`'s modification time `seconds` from now, so a test's two
/// writes never share one.
fn touch(path: &Path, seconds: u64) {
    let file = fs::File::options().write(true).open(path).unwrap();
    let time = std::time::SystemTime::now() + std::time::Duration::from_secs(seconds);
    file.set_modified(time).unwrap();
}

#[test]
fn edits_that_leave_gits_output_the_same_still_show() {
    let dir = repository();
    fs::write(dir.join("new.txt"), "a\n").unwrap();
    fs::write(dir.join("a.txt"), "one\n2\n").unwrap();
    let (mut changes, host) = following(&dir);
    changes.receive(&host, "changes", Data::map([("diff", "a.txt".into())]));
    changes.snapshot(true).unwrap();

    // An untracked file grows: git's status and numstat say the same.
    fs::write(dir.join("new.txt"), "a\nb\nc\n").unwrap();
    touch(&dir.join("new.txt"), 10);
    let snapshot = changes.snapshot(false).expect("the edit is seen");
    let files = snapshot["files"].as_array().unwrap();
    assert!(files.contains(&json!({ "path": "new.txt", "status": "A", "added": 3, "deleted": 0 })));

    // A tracked file changes text but not its totals.
    fs::write(dir.join("a.txt"), "one\n3\n").unwrap();
    touch(&dir.join("a.txt"), 20);
    let snapshot = changes.snapshot(false).expect("the edit is seen");
    let lines = &snapshot["diff"]["hunks"][0]["lines"];
    assert!(
        lines
            .as_array()
            .unwrap()
            .contains(&json!({ "kind": "add", "new": 2, "text": "3" }))
    );
    assert!(changes.snapshot(false).is_none());
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_path_is_one_file_never_a_pattern() {
    // `[ab].txt` is a name on every system, and as a pattern it also
    // matches `a.txt`, which `repository()` commits.
    let dir = repository();
    fs::write(dir.join("[ab].txt"), "star\n").unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "two"]);
    fs::write(dir.join("[ab].txt"), "star\nmore\n").unwrap();
    fs::write(dir.join("a.txt"), "one\nother\n").unwrap();
    let (mut changes, host) = following(&dir);
    changes.receive(&host, "changes", Data::map([("diff", "[ab].txt".into())]));
    let snapshot = changes.snapshot(true).unwrap();
    let hunks = snapshot["diff"]["hunks"].as_array().unwrap();
    assert_eq!(hunks.len(), 1, "{snapshot}");
    let texts: Vec<&str> = hunks[0]["lines"]
        .as_array()
        .unwrap()
        .iter()
        .map(|line| line["text"].as_str().unwrap())
        .collect();
    assert_eq!(texts, ["star", "more"]);
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_folder_made_a_repository_later_is_found() {
    let dir = tempdir();
    let (mut changes, _host) = following(&dir);
    assert_eq!(changes.snapshot(true).unwrap()["repository"], false);
    git(&dir, &["init", "-q"]);
    assert_eq!(changes.snapshot(false).unwrap()["repository"], true);
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_new_file_longer_than_what_is_read_says_so() {
    let dir = repository();
    // Few lines, many bytes: the line cap alone would not see the cut.
    let line = "x".repeat(1000);
    let text = format!("{line}\n").repeat(UNTRACKED_BYTES / 1000 + 10);
    fs::write(dir.join("big.txt"), text).unwrap();
    let (mut changes, host) = following(&dir);
    changes.receive(&host, "changes", Data::map([("diff", "big.txt".into())]));
    let snapshot = changes.snapshot(true).unwrap();
    assert_eq!(snapshot["partial"], true);
    assert_eq!(snapshot["diff"]["truncated"], true);
    fs::write(dir.join("big.txt"), "small\n").unwrap();
    touch(&dir.join("big.txt"), 10);
    let snapshot = changes.snapshot(false).unwrap();
    assert_eq!(snapshot["partial"], false);
    assert_eq!(snapshot["diff"]["truncated"], false);
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn switching_branches_in_a_clean_repository_shows_the_new_one() {
    let dir = repository();
    let (mut changes, _host) = following(&dir);
    let first = changes.snapshot(true).unwrap()["branch"].clone();
    git(&dir, &["switch", "-q", "-c", "other"]);
    let snapshot = changes.snapshot(false).expect("the switch is seen");
    assert_eq!(snapshot["branch"], "other");
    assert_ne!(snapshot["branch"], first);
    assert!(changes.snapshot(false).is_none());
    fs::remove_dir_all(&dir).unwrap();
}
