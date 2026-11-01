//! Moonmux, on Moonframe: a desktop app for terminals and coding agents whose
//! whole window (spaces, tabs, split panes, the agents list) is laid out by
//! the Luau in `app/`. Terminals run in herdr-core (`hc`), so they keep
//! running when the window closes; Rust adds `app.core`, what herdr-core
//! runs and which agents need you.
//!
//! `cargo run --release` opens it; `-- --check` loads it headlessly; edits
//! to `app/` show live.

mod core;
mod programs;

use {
    moonframe::Moonframe,
    std::{path::Path, process::ExitCode},
};

/// The app's name: changing it orphans running panes and loses saved
/// spaces. Its folders are `~/.config/moonmux` and
/// `~/.cache/moonmux`, and the `moonframe-app` label herdr-core
/// keeps on its terminals.
pub(crate) const NAME: &str = "moonmux";

/// `app/`, compiled into the binary: what a copy installed anywhere runs.
#[derive(moonframe::rust_embed::RustEmbed)]
#[crate_path = "moonframe::rust_embed"]
#[folder = "app"]
#[exclude = ".DS_Store"]
#[exclude = "*/.DS_Store"]
struct AppFiles;

/// `app/` in this checkout.
const SOURCE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/app");

/// Whether this binary runs from this checkout's `target/` (`cargo run`,
/// `just run`): then `app/` is read from disk and edits show live. A copy
/// installed or packaged elsewhere runs the app compiled into it.
fn from_checkout() -> bool {
    let target = Path::new(env!("CARGO_MANIFEST_DIR")).join("target");
    match (
        std::env::current_exe().and_then(|exe| exe.canonicalize()),
        target.canonicalize(),
    ) {
        (Ok(exe), Ok(target)) => exe.starts_with(target) && Path::new(SOURCE).is_dir(),
        _ => false,
    }
}

fn main() -> ExitCode {
    let app = Moonframe::new(NAME)
        .title("Moonmux")
        .app_id("dev.penso.moonmux")
        .version(env!("CARGO_PKG_VERSION"))
        .icon(include_bytes!("../app/icon.png"));
    let app = if from_checkout() {
        app.app_dir(SOURCE)
    } else {
        app.embedded_app::<AppFiles>()
    };
    app
        // Send feedback: the `?` that asks a coding agent to change this app.
        .keep(["feedback"])
        .native("core", core::Core::new)
        .run()
}
