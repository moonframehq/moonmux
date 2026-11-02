use {
    super::{agents::Status, *},
    crate::core::counts,
};

#[test]
fn pane_ids_come_from_the_session_name() {
    assert_eq!(pane_of("pane_p12+term"), "p12");
    assert_eq!(pane_of("pane_p12+term~00ff00ff00ff00ff"), "p12");
    assert_eq!(pane_of("pane_p12+termx"), "");
    assert_eq!(pane_of("pane_+term"), "");
    assert_eq!(pane_of("pane_p1_x+term"), "");
    assert_eq!(pane_of("work"), "");
    assert_eq!(pane_of(""), "");
}

fn pane(server: &str, id: &str, command: &str) -> Pane {
    Pane {
        server: server.into(),
        id: id.into(),
        session: "work".into(),
        window: "1".into(),
        index: "0".into(),
        command: command.into(),
        cwd: "/home/u/x".into(),
        ..Pane::default()
    }
}

#[test]
fn entries_and_counts() {
    let working = Track {
        status: Status::Working,
        since: 1_793_567_863_379,
        turn: 0,
        ..Track::default()
    };
    let blocked = Track {
        status: Status::Blocked,
        since: 5,
        turn: 2,
        ..Track::default()
    };
    let entries = [
        entry(
            &pane("moonmux", "%1", "zsh"),
            None,
            None,
            true,
            "p1",
            "/home/u",
        ),
        entry(
            &pane("moonmux", "%2", "node"),
            Some("codex"),
            Some(working),
            true,
            "p2",
            "/home/u",
        ),
        entry(
            &pane("default", "%7", "claude"),
            Some("claude"),
            Some(blocked),
            false,
            "",
            "/home/u",
        ),
    ];
    assert_eq!(entries[0]["cwd"], "~/x");
    assert_eq!(entries[0]["is_agent"], false);
    assert_eq!(entries[0]["status"], "");
    assert_eq!(entries[0]["status_ms"], 0);
    assert_eq!(entries[0]["target"], "%1");
    assert_eq!(entries[0]["backend"], "tmux");
    assert_eq!(entries[0]["program_key"], "shell");
    assert_eq!(entries[1]["agent"], "codex");
    assert_eq!(entries[1]["program"], "Codex");
    assert_eq!(entries[1]["status"], "working");
    assert_eq!(entries[1]["status_ms"], 1_793_567_863_379_i64);
    assert_eq!(entries[1]["pane"], "p2");
    assert_eq!(entries[2]["address"], "default:%7");
    assert_eq!(entries[2]["target"], "work:1.0");
    assert_eq!(entries[2]["program_key"], "claude");
    assert_eq!(entries[2]["turn"], 2);
    let counts = counts(&entries);
    assert_eq!(counts["live"], 3);
    assert_eq!(counts["agents"], 2);
    assert_eq!(counts["working"], 1);
    assert_eq!(counts["blocked"], 1);
    assert_eq!(counts["done"], 0);
}

#[test]
fn ours_sort_first_then_by_pane_number() {
    let mut entries = [
        entry(&pane("default", "%3", "zsh"), None, None, false, "", ""),
        entry(&pane("moonmux", "%10", "zsh"), None, None, true, "p2", ""),
        entry(&pane("moonmux", "%9", "zsh"), None, None, true, "p1", ""),
    ];
    entries.sort_by_key(sort_key);
    let order: Vec<&str> = entries
        .iter()
        .map(|entry| entry["address"].as_str().unwrap())
        .collect();
    assert_eq!(order, ["moonmux:%9", "moonmux:%10", "default:%3"]);
}

#[test]
fn a_pane_has_ended_until_its_shell_starts_again() {
    let seen: HashSet<String> = ["p2", "p3"].map(str::to_owned).into();
    let live: HashSet<String> = ["p2"].map(str::to_owned).into();
    assert_eq!(ended_panes(&seen, &live), vec!["p3".to_owned()]);
}

#[test]
fn a_new_agent_in_the_same_pane_counts_turns_on() {
    use agents::Seen;
    let done = Track::default()
        .next(Seen::Working, true, 1)
        .next(Seen::Quiet, false, 2);
    assert_eq!(done.turn, 1);
    // The agent exits: the pane is back at its shell.
    let shell = kept(done, None).unwrap();
    assert_eq!((shell.status, shell.turn), (Status::Idle, 1));
    // The next agent there finishes a turn of its own.
    let next = shell
        .next(Seen::Working, true, 3)
        .next(Seen::Quiet, false, 4);
    assert_eq!((next.status, next.turn), (Status::Done, 2));
    assert_eq!(kept(Track::default(), None), None);
    assert_eq!(kept(Track::default(), Some(next)), Some(next));
}

#[test]
fn only_known_keys_are_sent() {
    assert_eq!(tmux_key("esc"), Some("Escape"));
    assert_eq!(tmux_key("enter"), Some("Enter"));
    assert_eq!(tmux_key("C-c; kill-server"), None);
}

#[test]
fn its_addresses_are_server_and_pane() {
    let tmux = Tmux::new();
    assert!(tmux.owns("default:%3"));
    assert!(!tmux.owns("t7"));
    assert!(!tmux.owns("../x:%3"));
}

#[test]
fn a_missing_tmux_says_so() {
    let mut tmux = Tmux {
        tmux: None,
        ..Tmux::new()
    };
    assert!(!tmux.installed());
    let error = tmux.list(0, "/home/u").unwrap_err();
    assert!(error.contains("not installed"), "{error}");
    assert!(tmux.on_pane("default:%1", &[&["send-keys", "x"]]).is_err());
    assert!(tmux.start("p1", "claude", "claude").is_err());
}

#[test]
fn unknown_servers_are_refused() {
    let tmux = Tmux {
        socket_dir: None,
        ..Tmux::new()
    };
    let error = tmux
        .on_pane("someone-else:%1", &[&["send-keys", "x"]])
        .unwrap_err();
    assert!(
        error.contains("someone-else") || error.contains("not installed"),
        "{error}"
    );
    assert!(tmux.on_pane("not an address", &[]).is_err());
}

/// A tmux server of the test's own, killed when it ends.
struct Server {
    tmux: Program,
    name: String,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.tmux.run(&self.name, &["kill-server"]);
    }
}

#[test]
fn panes_on_a_real_server_are_found_typed_into_and_closed() {
    let Some(program) = Program::find() else {
        eprintln!("skipped: tmux is not installed");
        return;
    };
    let server = Server {
        tmux: program.clone(),
        name: format!("moonmux-test-{}", std::process::id()),
    };
    let start = |session: &str| {
        program
            .run(
                &server.name,
                &[
                    "-f",
                    "/dev/null",
                    "new-session",
                    "-d",
                    "-s",
                    session,
                    "-x",
                    "80",
                    "-y",
                    "24",
                    "sh",
                ],
            )
            .map_err(|failure| failure_text(&failure))
    };
    // tmux's `-f` comes before the command; `run` puts `-L` first.
    if let Err(error) = start("pane_p3+term~00ff00ff00ff00ff") {
        panic!("tmux could not start a session: {error}");
    }
    start("pane_p4+term").unwrap();
    start("scratch").unwrap();

    let mut provider = Tmux {
        tmux: Some(program.clone()),
        socket: server.name.clone(),
        socket_dir: None,
        ..Tmux::new()
    };
    let listing = provider.list(0, "/home/u").unwrap();
    let terminals = &listing.terminals;
    let panes: Vec<&str> = terminals
        .iter()
        .map(|terminal| terminal["pane"].as_str().unwrap())
        .collect();
    assert!(panes.contains(&"p3"), "{terminals:?}");
    assert!(panes.contains(&"p4"), "{terminals:?}");
    assert!(
        panes.contains(&""),
        "the scratch session is ours but no pane's"
    );
    assert_eq!(terminals.len(), 3);
    assert!(terminals.iter().all(|terminal| terminal["ours"] == true));
    assert!(listing.finished.is_empty());

    // Typing into p3, then reading its screen as a peek does.
    let p3 = terminals
        .iter()
        .find(|terminal| terminal["pane"] == "p3")
        .unwrap()["address"]
        .as_str()
        .unwrap()
        .to_owned();
    provider.send(&p3, "echo moon-$((6*7))").unwrap();
    let mut seen = String::new();
    for _ in 0..50 {
        seen = provider.read(&p3).unwrap_or_default();
        if seen.contains("moon-42") {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    assert!(seen.contains("moon-42"), "{seen}");

    // An agent is typed into p4 once: the session is marked first.
    let typed = provider.start("p4", "claude", "echo agent-$((2*21))");
    assert_eq!(typed, Ok(Some(String::new())));
    let again = provider.start("p4", "claude", "echo twice");
    assert_eq!(again, Ok(Some("already started".to_owned())));
    assert_eq!(provider.start("p9", "claude", "echo none"), Ok(None));

    // Closing p3 ends its session and no other.
    assert_eq!(provider.close_panes(&["p3"]), Ok(String::new()));
    let sessions = program.sessions(&server.name).unwrap();
    assert!(
        !sessions
            .iter()
            .any(|session| session.starts_with("pane_p3"))
    );
    assert!(sessions.contains(&"pane_p4+term".to_owned()));

    // A shell that exits leaves its pane ended.
    program
        .run(&server.name, &["kill-session", "-t", "=pane_p4+term"])
        .unwrap();
    let listing = provider.list(1, "/home/u").unwrap();
    assert_eq!(listing.finished, ["p4"]);
}
