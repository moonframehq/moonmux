use {
    super::*,
    std::sync::{Arc, Mutex},
};

/// A backend that lists what it is given and writes down what it is told.
struct Fake {
    id: &'static str,
    installed: bool,
    /// Its addresses start with this.
    prefix: &'static str,
    listing: Result<Vec<Value>, String>,
    finished: Vec<String>,
    told: Arc<Mutex<Vec<String>>>,
}

impl Fake {
    fn new(id: &'static str, prefix: &'static str, told: &Arc<Mutex<Vec<String>>>) -> Self {
        Self {
            id,
            installed: true,
            prefix,
            listing: Ok(Vec::new()),
            finished: Vec::new(),
            told: Arc::clone(told),
        }
    }

    fn tell(&self, what: String) -> Result<String, String> {
        self.told
            .lock()
            .unwrap()
            .push(format!("{} {what}", self.id));
        Ok(String::new())
    }
}

impl Backend for Fake {
    fn id(&self) -> &'static str {
        self.id
    }
    fn installed(&self) -> bool {
        self.installed
    }
    fn find(&mut self) {}
    fn info(&self) -> Value {
        json!({ "name": "here", "version": "1.0" })
    }
    fn list(&mut self, _ticks: u64, _home: &str) -> Result<Listing, String> {
        Ok(Listing {
            terminals: self.listing.clone()?,
            finished: self.finished.clone(),
        })
    }
    fn owns(&self, address: &str) -> bool {
        address.starts_with(self.prefix)
    }
    fn read(&mut self, address: &str) -> Result<String, String> {
        Ok(format!("screen of {address}"))
    }
    fn send(&mut self, address: &str, text: &str) -> Result<String, String> {
        self.tell(format!("send {address} {text}"))
    }
    fn keys(&mut self, address: &str, key: &str) -> Result<String, String> {
        self.tell(format!("keys {address} {key}"))
    }
    fn interrupt(&mut self, address: &str) -> Result<String, String> {
        self.tell(format!("interrupt {address}"))
    }
    fn close_panes(&mut self, panes: &[&str]) -> Result<String, String> {
        self.tell(format!("close {}", panes.join(",")))
    }
    fn start(&mut self, _pane: &str, _key: &str, _command: &str) -> Result<Option<String>, String> {
        Ok(None)
    }
}

fn terminal(address: &str, pane: &str, status: &str) -> Value {
    json!({ "address": address, "pane": pane, "ours": !pane.is_empty(), "status": status, "is_agent": !status.is_empty() })
}

fn two(told: &Arc<Mutex<Vec<String>>>) -> (Fake, Fake) {
    (
        Fake::new("herdr-core", "t", told),
        Fake::new("tmux", "default:", told),
    )
}

#[test]
fn both_backends_are_listed_together() {
    let told = Arc::default();
    let (mut herdr, mut tmux) = two(&told);
    herdr.listing = Ok(vec![terminal("t1", "p1", "working")]);
    herdr.finished = vec!["p2".into(), "p3".into()];
    tmux.listing = Ok(vec![
        terminal("default:%3", "p3", ""),
        terminal("default:%4", "", "blocked"),
    ]);
    let mut core = Core::with(vec![Box::new(herdr), Box::new(tmux)]);
    let snapshot = core.snapshot();
    assert_eq!(snapshot["ok"], true, "{snapshot}");
    assert_eq!(snapshot["error"], "");
    assert_eq!(snapshot["counts"]["live"], 3);
    assert_eq!(snapshot["counts"]["working"], 1);
    assert_eq!(snapshot["counts"]["blocked"], 1);
    // p3 finished on herdr-core but lives on tmux: it has not ended.
    assert_eq!(snapshot["ended"], json!(["p2"]));
    assert_eq!(snapshot["backends"]["auto"], "herdr-core");
    let ids: Vec<&str> = snapshot["backends"]["list"]
        .as_array()
        .unwrap()
        .iter()
        .map(|backend| backend["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["herdr-core", "tmux"]);
    assert_eq!(snapshot["core"]["version"], "1.0");
}

#[test]
fn auto_is_tmux_without_herdr_core() {
    let told = Arc::default();
    let (mut herdr, tmux) = two(&told);
    herdr.installed = false;
    herdr.listing = Err("herdr-core is not installed".into());
    let mut core = Core::with(vec![Box::new(herdr), Box::new(tmux)]);
    let snapshot = core.snapshot();
    assert_eq!(snapshot["backends"]["auto"], "tmux");
    // One that is not installed is no error while another works.
    assert_eq!(snapshot["ok"], true);
    assert_eq!(snapshot["error"], "");
    assert_eq!(snapshot["core"], Value::Null);
    assert_eq!(snapshot["backends"]["list"][0]["installed"], false);
}

#[test]
fn an_installed_backend_failing_says_why_but_the_other_still_shows() {
    let told = Arc::default();
    let (mut herdr, mut tmux) = two(&told);
    herdr.listing = Err("hc: connection refused".into());
    tmux.listing = Ok(vec![terminal("default:%1", "p1", "")]);
    let mut core = Core::with(vec![Box::new(herdr), Box::new(tmux)]);
    let snapshot = core.snapshot();
    assert_eq!(snapshot["ok"], true);
    assert_eq!(snapshot["error"], "hc: connection refused");
    assert_eq!(snapshot["counts"]["live"], 1);
    assert_eq!(snapshot["backends"]["list"][0]["ok"], false);
}

#[test]
fn with_no_backend_installed_nothing_is_listed() {
    let told = Arc::default();
    let (mut herdr, mut tmux) = two(&told);
    herdr.installed = false;
    herdr.listing = Err("herdr-core is not installed".into());
    tmux.installed = false;
    tmux.listing = Err("tmux is not installed".into());
    let mut core = Core::with(vec![Box::new(herdr), Box::new(tmux)]);
    let snapshot = core.snapshot();
    assert_eq!(snapshot["ok"], false);
    assert_eq!(
        snapshot["error"],
        "herdr-core is not installed; tmux is not installed"
    );
    assert_eq!(snapshot["backends"]["auto"], "");
    assert_eq!(snapshot["terminals"], json!([]));
    assert_eq!(snapshot["machines"]["ok"], false);
}

#[test]
fn actions_go_to_the_backend_that_owns_the_address() {
    let told: Arc<Mutex<Vec<String>>> = Arc::default();
    let (herdr, tmux) = two(&told);
    let mut core = Core::with(vec![Box::new(herdr), Box::new(tmux)]);
    let send = Data::map([("send", "default:%3".into()), ("text", "hi".into())]);
    assert!(matches!(core.act(&send), Some(("send", _, Ok(_)))));
    let keys = Data::map([("keys", "t7".into()), ("key", "esc".into())]);
    assert!(matches!(core.act(&keys), Some(("keys", _, Ok(_)))));
    let interrupt = Data::map([("interrupt", "elsewhere".into())]);
    let (_, _, result) = core.act(&interrupt).unwrap();
    assert!(result.unwrap_err().contains("not a terminal"));
    core.act(&Data::map([("close_panes", "p1,p4".into())]));
    assert_eq!(
        *told.lock().unwrap(),
        [
            "tmux send default:%3 hi",
            "herdr-core keys t7 esc",
            "herdr-core close p1,p4",
            "tmux close p1,p4",
        ]
    );
    // A peek reads from the owner too.
    core.watching = "default:%3".into();
    assert_eq!(core.snapshot()["peek"]["text"], "screen of default:%3");
}

#[test]
fn start_refuses_what_is_not_a_startable_agent() {
    let told = Arc::default();
    let (herdr, tmux) = two(&told);
    let mut core = Core::with(vec![Box::new(herdr), Box::new(tmux)]);
    let error = core.start("p1", "rm -rf ~").unwrap_err();
    assert!(error.contains("not an agent"), "{error}");
    let message = Data::map([("start", "p1".into()), ("program", "amp".into())]);
    let (action, pane, result) = core.act(&message).unwrap();
    assert_eq!((action, pane.as_str()), ("start", "p1"));
    assert!(result.is_err());
}

#[test]
fn folders_under_home_get_a_tilde() {
    assert_eq!(tilde("/Users/me", "/Users/me"), "~");
    assert_eq!(tilde("/Users/me/", "/Users/me"), "~");
    assert_eq!(tilde("/Users/me/code/x", "/Users/me/"), "~/code/x");
    assert_eq!(tilde("/Users/meow", "/Users/me"), "/Users/meow");
    assert_eq!(tilde("/tmp", ""), "/tmp");
}

#[test]
fn the_clock_is_published_by_the_minute() {
    assert_eq!(to_minute(1_793_567_863_379), 1_793_567_820_000);
    assert_eq!(to_minute(1_793_567_820_000), 1_793_567_820_000);
    assert_eq!(to_minute(59_999), 0);
    assert!(now_ms() > 1_700_000_000_000);
}

#[test]
fn only_the_named_writers_may_act() {
    let settings = Data::map([("writers", vec!["peek", "workspace"].into())]);
    let writers = writers(&settings);
    assert!(may_act(&writers, "peek"));
    assert!(may_act(&writers, "workspace"));
    assert!(!may_act(&writers, "sidebar"));
    assert!(!may_act(&writers, "someones-package"));
}

#[test]
fn without_writers_no_mod_may_act() {
    let none = writers(&Data::Nil);
    assert!(none.is_empty());
    assert!(!may_act(&none, "peek"));
}

#[test]
fn a_pane_has_ended_until_its_shell_starts_again() {
    let live = [json!({ "pane": "p2" })];
    let finished = vec!["p3".to_owned(), "p2".to_owned(), "p3".to_owned()];
    assert_eq!(ended_panes(finished, &live), vec!["p3".to_owned()]);
}

#[test]
fn only_the_releases_page_opens() {
    assert_eq!(page_address("releases"), Some(RELEASES));
    assert_eq!(page_address("https://example.com"), None);
    assert!(open_page("file:///etc/passwd").is_err());
}

#[test]
#[cfg(not(windows))]
fn a_failed_opener_is_reported() {
    assert!(opened("open", true, Some(0)).is_ok());
    let error = opened("xdg-open", false, Some(3)).unwrap_err();
    assert!(error.contains("exit 3"), "{error}");
    assert!(opened("open", false, None).is_err());
}

#[test]
fn requests_any_mod_may_send_run_once_a_tick() {
    let mut last = None;
    assert!(once_a_tick(&mut last, 4));
    assert!(!once_a_tick(&mut last, 4));
    assert!(once_a_tick(&mut last, 5));
}

#[test]
fn replies_carry_the_action_and_its_outcome() {
    let ok = reply("open", "releases", Ok(String::new()));
    assert_eq!(ok.get("reply").and_then(Data::as_str), Some("open"));
    assert_eq!(ok.get("ok"), Some(&Data::Bool(true)));
    let failed = reply("send", "t1", Err("gone".to_owned()));
    assert_eq!(failed.get("ok"), Some(&Data::Bool(false)));
    assert_eq!(failed.get("error").and_then(Data::as_str), Some("gone"));
}
