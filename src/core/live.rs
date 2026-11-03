//! What `app.core` last published, shared between the provider's thread
//! and a backend's own (herdr-core's `hc events`), so that a pane's shell
//! changing folder is published the moment the backend hears of it, not at
//! the next tick: the Files and Changes panels follow that folder.
//!
//! A folder heard between ticks stays what is published for its terminal
//! until the terminal is gone or the backend says it may have missed some
//! (`forget`): a listing begun just before a `cd` would otherwise put the
//! folder it left back for a tick.

use {
    super::tilde,
    moonframe::Handle,
    serde_json::Value,
    std::{
        collections::HashMap,
        sync::{Arc, Mutex, MutexGuard, PoisonError},
    },
};

/// Bytes of a folder a backend may report: a longer one is not a folder.
// Only herdr-core hears moves between ticks.
#[cfg_attr(not(feature = "herdr-core"), allow(dead_code))]
const MAX_CWD: usize = 4096;

#[derive(Default)]
struct Shared {
    /// What was last published, so an unchanged one is not published again.
    last: Value,
    /// The folder heard for each terminal since it was last listed, by
    /// address.
    cwds: HashMap<String, String>,
}

/// The provider's handle on what it publishes, from any thread.
#[derive(Clone)]
pub(crate) struct Live {
    shared: Arc<Mutex<Shared>>,
    host: Handle,
    #[cfg_attr(not(feature = "herdr-core"), allow(dead_code))]
    home: String,
}

impl Live {
    pub(crate) fn new(host: &Handle, home: &str) -> Self {
        Self {
            shared: Arc::default(),
            host: host.clone(),
            home: home.to_owned(),
        }
    }

    fn lock(&self) -> MutexGuard<'_, Shared> {
        self.shared.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Publishes `snapshot`, with the folders heard since put in, unless it
    /// is what was last published.
    pub(crate) fn publish(&self, mut snapshot: Value) {
        let mut shared = self.lock();
        let live: Vec<&str> = terminals(&snapshot)
            .iter()
            .filter_map(|terminal| terminal.get("address").and_then(Value::as_str))
            .collect();
        shared
            .cwds
            .retain(|address, _| live.contains(&address.as_str()));
        for (address, cwd) in &shared.cwds {
            set_cwd(&mut snapshot, address, cwd);
        }
        if snapshot != shared.last {
            self.host.publish_json(&snapshot);
            shared.last = snapshot;
        }
    }

    #[cfg_attr(not(feature = "herdr-core"), allow(dead_code))]
    /// The shell of the terminal `address` is now in `cwd`, as its backend
    /// just heard: published at once.
    pub(crate) fn moved(&self, address: &str, cwd: &str) {
        if address.is_empty() || cwd.is_empty() || cwd.len() > MAX_CWD {
            return;
        }
        let cwd = tilde(cwd, &self.home);
        let mut shared = self.lock();
        shared.cwds.insert(address.to_owned(), cwd.clone());
        let mut snapshot = shared.last.clone();
        if set_cwd(&mut snapshot, address, &cwd) {
            self.host.publish_json(&snapshot);
            shared.last = snapshot;
        }
    }

    #[cfg_attr(not(feature = "herdr-core"), allow(dead_code))]
    /// The backend may have missed some moves (its stream ended): the next
    /// listing says where each shell is.
    pub(crate) fn forget(&self) {
        self.lock().cwds.clear();
    }
}

fn terminals(snapshot: &Value) -> &[Value] {
    snapshot
        .get("terminals")
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

/// Sets the folder of the terminal `address` in `snapshot`; whether that
/// changed it.
fn set_cwd(snapshot: &mut Value, address: &str, cwd: &str) -> bool {
    let Some(terminal) = snapshot
        .get_mut("terminals")
        .and_then(Value::as_array_mut)
        .and_then(|terminals| {
            terminals
                .iter_mut()
                .find(|terminal| terminal.get("address").and_then(Value::as_str) == Some(address))
        })
    else {
        return false;
    };
    if terminal.get("cwd").and_then(Value::as_str) == Some(cwd) {
        return false;
    }
    terminal["cwd"] = Value::String(cwd.to_owned());
    true
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use {
        super::*,
        moonframe::Data,
        serde_json::json,
        std::{
            path::Path,
            sync::atomic::{AtomicUsize, Ordering},
        },
    };

    /// A `Live`, and how many times it published.
    fn live() -> (Live, Arc<AtomicUsize>) {
        let published = Arc::new(AtomicUsize::new(0));
        let count = Arc::clone(&published);
        let host = Handle::new(
            "core",
            Path::new("."),
            Data::Nil,
            Arc::new(move |_| {
                count.fetch_add(1, Ordering::SeqCst);
            }),
        );
        (Live::new(&host, "/home/u"), published)
    }

    /// The folder of each terminal last published.
    fn cwds(live: &Live) -> Vec<(String, String)> {
        terminals(&live.lock().last)
            .iter()
            .map(|terminal| {
                let field = |key| {
                    terminal
                        .get(key)
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_owned()
                };
                (field("address"), field("cwd"))
            })
            .collect()
    }

    fn snapshot(cwd: &str) -> Value {
        json!({ "ok": true, "terminals": [
            { "address": "t1", "cwd": cwd, "ours": true },
            { "address": "t2", "cwd": "~", "ours": true },
        ] })
    }

    fn pair(address: &str, cwd: &str) -> (String, String) {
        (address.to_owned(), cwd.to_owned())
    }

    #[test]
    fn a_move_is_published_at_once_and_outlives_an_older_listing() {
        let (live, published) = live();
        live.publish(snapshot("~/code"));
        live.moved("t1", "/home/u/code/app");
        assert_eq!(published.load(Ordering::SeqCst), 2);
        assert_eq!(cwds(&live), vec![pair("t1", "~/code/app"), pair("t2", "~")]);
        // A listing that began before the `cd` still says the old folder.
        live.publish(snapshot("~/code"));
        assert_eq!(published.load(Ordering::SeqCst), 2, "it changed nothing");
        assert_eq!(cwds(&live)[0], pair("t1", "~/code/app"));
    }

    #[test]
    fn a_move_to_the_same_folder_or_an_unknown_terminal_publishes_nothing() {
        let (live, published) = live();
        live.publish(snapshot("~/code"));
        live.moved("t1", "/home/u/code");
        live.moved("t9", "/srv");
        live.moved("t1", "");
        live.moved("t1", &"x".repeat(MAX_CWD + 1));
        assert_eq!(published.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn forgetting_lets_the_listing_say_again() {
        let (live, _) = live();
        live.publish(snapshot("~/code"));
        live.moved("t1", "/home/u/code/app");
        live.forget();
        live.publish(snapshot("~/elsewhere"));
        assert_eq!(cwds(&live)[0], pair("t1", "~/elsewhere"));
    }

    #[test]
    fn a_gone_terminal_is_forgotten() {
        let (live, _) = live();
        live.publish(snapshot("~/code"));
        live.moved("t1", "/home/u/code/app");
        // t1 is gone, then its address is given to a new terminal.
        live.publish(json!({ "ok": true, "terminals": [] }));
        live.publish(snapshot("~/new"));
        assert_eq!(cwds(&live)[0], pair("t1", "~/new"));
    }
}
