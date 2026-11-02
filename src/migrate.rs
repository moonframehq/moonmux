//! Bringing along what the app kept under its former name.
//!
//! The app was `herdr-moonframe` before it was [`crate::NAME`]. Its name
//! is where Moonframe keeps everything: the user's folder
//! (`~/.config/<name>`), the saved spaces (`~/.local/state/<name>`, or the
//! document `<name>` in herdr-core when herdr-core runs), and the
//! `moonframe-app=<name>` label on each herdr-core terminal, by which a pane
//! finds its running shell again. So before the window opens, each of these
//! is moved to the new name, once: only when the new name has nothing yet,
//! so nothing the app saved under it is ever written over. The tmux
//! backend came with the new name; nothing of it is under the former one.
//!
//! Each step that fails is logged and skipped: the app then starts as on a
//! first launch, and the old copy stays where it was.

use std::{
    env,
    path::{Path, PathBuf},
};

/// The app's name before it was [`crate::NAME`].
const FORMER: &str = "herdr-moonframe";

/// Moves what the former name kept to the current one, where the current
/// one has nothing yet.
pub(crate) fn run() {
    for (variable, fallback) in [
        ("XDG_CONFIG_HOME", ".config"),
        ("XDG_STATE_HOME", ".local/state"),
    ] {
        let Some(base) = xdg(variable, fallback) else {
            continue;
        };
        if let Err(error) = move_folder(&base.join(FORMER), &base.join(crate::NAME)) {
            tracing::warn!(
                "cannot bring {FORMER}'s folder in {}: {error}",
                base.display()
            );
        }
    }
    #[cfg(feature = "herdr-core")]
    herdr::run();
}

/// `$variable`, else `~/fallback`, as Moonframe finds its folders.
fn xdg(variable: &str, fallback: &str) -> Option<PathBuf> {
    env::var_os(variable)
        .map(PathBuf::from)
        // The spec ignores relative values.
        .filter(|dir| dir.is_absolute())
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(fallback)))
}

/// Renames `from` to `to` when `from` is a folder and nothing is at `to`.
/// Whether it moved.
fn move_folder(from: &Path, to: &Path) -> std::io::Result<bool> {
    if !from.is_dir() || to.symlink_metadata().is_ok() {
        return Ok(false);
    }
    std::fs::rename(from, to)?;
    tracing::info!("moved {} to {}", from.display(), to.display());
    Ok(true)
}

/// herdr-core's side: the saved document, and the terminals' label.
#[cfg(feature = "herdr-core")]
mod herdr {
    use {
        super::FORMER,
        crate::core::{new_command, output_within},
        serde_json::{Value, json},
        std::{path::Path, time::Duration},
    };

    /// How long one `hc` call may take: the window waits for these.
    const DEADLINE: Duration = Duration::from_secs(3);
    /// The label naming the app a terminal belongs to.
    const APP_LABEL: &str = "moonframe-app";

    pub(super) fn run() {
        let Some(hc) = crate::core::find_hc() else {
            return;
        };
        if let Err(error) = move_document(&hc) {
            tracing::warn!("cannot bring {FORMER}'s saved spaces from herdr-core: {error}");
        }
        if let Err(error) = relabel(&hc) {
            tracing::warn!("cannot bring {FORMER}'s terminals in herdr-core: {error}");
        }
    }

    /// Copies the document `herdr-moonframe` to the app's name, when that
    /// was never written. The old one stays, for an older build.
    fn move_document(hc: &Path) -> Result<(), String> {
        let answers = call(hc, &[("doc.get", json!({ "name": FORMER }))])?;
        let Some(value) = answers.first().and_then(|doc| former_document(doc)) else {
            return Ok(());
        };
        // Version 0 writes only a document never written: one the app saved
        // under its new name is never written over.
        let set = json!({ "name": crate::NAME, "value": value, "version": 0 });
        match call(hc, &[("doc.set", set)]) {
            Ok(_) => {
                tracing::info!("moved {FORMER}'s saved spaces to {}", crate::NAME);
                Ok(())
            }
            Err(error) if error.contains("conflict") => Ok(()),
            Err(error) => Err(error),
        }
    }

    /// The value of a `doc.get` answer, when the document was written.
    pub(super) fn former_document(doc: &Value) -> Option<&Value> {
        let written = doc.get("version").and_then(Value::as_u64).unwrap_or(0) > 0;
        doc.get("value").filter(|value| written && !value.is_null())
    }

    /// Labels the former app's live terminals as this app's, so each pane
    /// finds its running shell again.
    fn relabel(hc: &Path) -> Result<(), String> {
        let listed = hc_json(
            hc,
            &["ls", "--json", "--label", &format!("{APP_LABEL}={FORMER}")],
        )?;
        let label = format!("{APP_LABEL}={}", crate::NAME);
        let mut problems = Vec::new();
        for address in addresses(&listed) {
            if let Err(error) = hc_json(hc, &["label", address, &label, "--json"]) {
                problems.push(format!("{address}: {error}"));
            }
        }
        if problems.is_empty() {
            Ok(())
        } else {
            Err(problems.join("; "))
        }
    }

    /// The addresses `hc ls --json` lists.
    pub(super) fn addresses(listed: &Value) -> impl Iterator<Item = &str> {
        listed
            .get("terminals")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|terminal| terminal.get("address").and_then(Value::as_str))
    }

    /// Runs `hc args...` and reads what it printed as JSON.
    fn hc_json(hc: &Path, args: &[&str]) -> Result<Value, String> {
        let mut command = new_command(hc);
        command.args(args);
        let output = output_within(&mut command, b"", DEADLINE)
            .map_err(|error| format!("hc {}: {error}", args.join(" ")))?;
        if !output.status.success() {
            let said = String::from_utf8_lossy(&output.stderr).trim().to_owned();
            return Err(format!("hc {} failed: {said}", args.join(" ")));
        }
        serde_json::from_slice(&output.stdout)
            .map_err(|error| format!("hc {}: {error}", args.join(" ")))
    }

    /// Calls herdr-core's API through `hc connect`, after its handshake:
    /// the result of each of `calls`, in order.
    fn call(hc: &Path, calls: &[(&str, Value)]) -> Result<Vec<Value>, String> {
        let hello = json!({ "jsonrpc": "2.0", "id": 0, "method": "hello",
                            "params": { "generation": 1 } });
        let mut input = Vec::new();
        for message in std::iter::once(hello).chain(calls.iter().enumerate().map(
            |(index, (method, params))| {
                json!({ "jsonrpc": "2.0", "id": index + 1, "method": method, "params": params })
            },
        )) {
            input.extend(message.to_string().into_bytes());
            input.push(b'\n');
        }
        let mut command = new_command(hc);
        command.arg("connect");
        let output = output_within(&mut command, &input, DEADLINE)
            .map_err(|error| format!("hc connect: {error}"))?;
        answers(&String::from_utf8_lossy(&output.stdout), calls.len())
    }

    /// The results of calls `1..=count` in what `hc connect` printed, or
    /// the first error.
    pub(super) fn answers(printed: &str, count: usize) -> Result<Vec<Value>, String> {
        let replies: Vec<Value> = printed
            .lines()
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect();
        (1..=count)
            .map(|id| {
                let reply = replies
                    .iter()
                    .find(|reply| reply.get("id").and_then(Value::as_u64) == Some(id as u64))
                    .ok_or_else(|| format!("herdr-core did not answer call {id}"))?;
                if let Some(error) = reply.get("error") {
                    let kind = error["data"]["kind"].as_str().unwrap_or("");
                    let message = error["message"].as_str().unwrap_or("failed");
                    return Err(if kind.is_empty() {
                        message.to_owned()
                    } else {
                        kind.to_owned()
                    });
                }
                Ok(reply.get("result").cloned().unwrap_or(Value::Null))
            })
            .collect()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = env::temp_dir().join(format!("moonmux-migrate-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn the_former_folder_moves_when_the_new_one_is_missing() {
        let dir = scratch("moves");
        std::fs::create_dir(dir.join(FORMER)).unwrap();
        std::fs::write(dir.join(FORMER).join("settings.toml"), "x").unwrap();
        assert!(move_folder(&dir.join(FORMER), &dir.join(crate::NAME)).unwrap());
        assert!(!dir.join(FORMER).exists());
        assert_eq!(
            std::fs::read_to_string(dir.join(crate::NAME).join("settings.toml")).unwrap(),
            "x"
        );
        // Once moved, there is nothing left to move.
        assert!(!move_folder(&dir.join(FORMER), &dir.join(crate::NAME)).unwrap());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_new_folder_is_never_written_over() {
        let dir = scratch("kept");
        std::fs::create_dir(dir.join(FORMER)).unwrap();
        std::fs::create_dir(dir.join(crate::NAME)).unwrap();
        std::fs::write(dir.join(crate::NAME).join("settings.toml"), "new").unwrap();
        assert!(!move_folder(&dir.join(FORMER), &dir.join(crate::NAME)).unwrap());
        assert!(dir.join(FORMER).is_dir());
        assert_eq!(
            std::fs::read_to_string(dir.join(crate::NAME).join("settings.toml")).unwrap(),
            "new"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(feature = "herdr-core")]
    #[test]
    fn only_a_written_document_is_brought_along() {
        use serde_json::json;
        let written = json!({ "value": { "format": "moonframe.document" }, "version": 4 });
        assert_eq!(
            herdr::former_document(&written),
            Some(&json!({ "format": "moonframe.document" }))
        );
        assert_eq!(
            herdr::former_document(&json!({ "value": null, "version": 0 })),
            None
        );
        assert_eq!(
            herdr::former_document(&json!({ "value": null, "version": 3 })),
            None
        );
    }

    #[cfg(feature = "herdr-core")]
    #[test]
    fn answers_are_matched_by_id_and_errors_name_their_kind() {
        let printed = concat!(
            r#"{"id":0,"jsonrpc":"2.0","result":{"generation":1}}"#,
            "\n",
            r#"{"id":2,"jsonrpc":"2.0","result":{"version":7}}"#,
            "\n",
            r#"{"id":1,"jsonrpc":"2.0","result":{"value":null,"version":0}}"#,
            "\n",
        );
        let found = herdr::answers(printed, 2).unwrap();
        assert_eq!(found[0]["version"], 0);
        assert_eq!(found[1]["version"], 7);
        let refused = r#"{"id":1,"jsonrpc":"2.0","error":{"code":-32000,"message":"conflict","data":{"kind":"conflict"}}}"#;
        assert_eq!(herdr::answers(refused, 1).unwrap_err(), "conflict");
        assert!(
            herdr::answers("", 1)
                .unwrap_err()
                .contains("did not answer")
        );
    }

    #[cfg(feature = "herdr-core")]
    #[test]
    fn listed_terminals_give_their_addresses() {
        let listed = serde_json::json!({ "terminals": [
            { "address": "t1q", "name": "herdr-moonframe.pane-p1.term" },
            { "name": "no address" },
            { "address": "t1s" },
        ] });
        assert_eq!(
            herdr::addresses(&listed).collect::<Vec<_>>(),
            ["t1q", "t1s"]
        );
    }
}
