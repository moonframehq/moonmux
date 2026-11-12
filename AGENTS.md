# AGENTS.md

Engineering guidance for agents working in this repository. Read this alongside
`README.md`, `docs/releasing.md`, and the code before making changes.

## Priorities

- Keep changes small, explicit, and focused on the root cause.
- Preserve user and other agents' changes. Never revert unrelated edits.
- Prefer existing standard traits and generics over unnecessarily concrete APIs.
- Protect bounded resource use and UI responsiveness.
- Add regression tests for changed behavior and report what was actually verified.
- Never modify herdr-core or Moonframe from here. herdr-core is owned by an
  external team; Moonframe is its own repository, followed on its `main`. Sibling
  checkouts are read-only references. Propose an upstream change only when
  genuinely justified.

## Architecture

Moonmux is a desktop app for terminals and coding agents (like herdr-app),
built on the Moonframe library. Its name (`NAME` in `src/main.rs`) is the
label on its herdr-core terminals, its tmux server (`tmux -L moonmux`), its
config and cache folders and its saved workspace: changing it, or the app
id (`dev.penso.moonmux`), orphans users' running panes and loses their
spaces, so a rename needs a migration (`src/migrate.rs` brings along
the former `herdr-moonframe`). A backend owns every terminal
process, herdr-core (`hc`) or tmux; this app arranges them and shows the
agents. Closing the window must leave the backends and their terminals
running.

Each backend is a cargo feature, both on by default: `herdr-core` (which
turns on Moonframe's `herdr-core` feature) and `tmux` (Moonframe's tmux
backend needs no feature). The app must build, pass clippy and pass its
tests with either alone and with both; `just features` and CI check that.

What draws the window is a feature as well: `gpui` (default) or `rune`,
Moonframe's experimental renderer (`just run-rune`). `src/main.rs` names
either crate `moonframe`, so the rest of the code never names one; never
use GPUI or Rune directly. Rune lacks windows of their own (Settings, Make
it yours) and named themes for now. A Rune build must also pass clippy,
its tests and `--check` (`just features`, `just check-rune`, CI).

| Location | Responsibility |
| --- | --- |
| `src/main.rs` | `Moonframe::new("moonmux")`: `app/` read live from the checkout, else compiled in (`AppFiles`) |
| `src/migrate.rs` | Before the window opens, moves what the app kept as `herdr-moonframe` (its folders, its herdr-core document, its terminals' label) to `NAME`, once, never over anything saved under it |
| `src/core/mod.rs` | The `core` native provider (`app.core`): each second, the terminals of every backend built in and installed, behind the `Backend` trait; which backend Auto picks; routing a peek and actions (send, keys, interrupt, close_panes, start) to the backend whose address they name; opening the releases page; `new_command` for every child process |
| `src/core/herdr.rs` | The herdr-core backend (`herdr-core` feature): `hc ls`, labels via `hc get`, a peek (`hc read`), machines (`hc machine ls`), `hc kill` and `hc label` |
| `src/core/tmux/` | The tmux backend (`tmux` feature): panes of the app's own server (`tmux -L moonmux`) and the user's other servers (`server.rs`), agents found under each pane's shell and read off its screen (`agents.rs`) |
| `app/main.luau` | The shell: sidebar, workspace card, status line, peek sheet; menus and their shortcuts |
| `app/mods/workspace` | Spaces, their tabs, and each tab's tree of split panes, all in its state, with the backend each pane runs on; the tab strip is the title bar. Publishes `topic:moonmux/workspace` |
| `app/mods/pane` | One instance per pane (`pane#p3`), each owning one terminal (`pane#p3/term`) on the backend the workspace gives it |
| `app/mods/sidebar` | Spaces, then agents in our panes, then agents elsewhere |
| `app/mods/peek`, `app/mods/statusbar` | The peek sheet and the status line |
| `app/mods/settings` | The Settings window (⌘,; its own window, through `app:window`, not mounted by the shell): Look (appearance and a light and a dark theme, through `app:appearance` and `app:theme`, and the layout choices it publishes on `topic:moonmux/layout`), Terminals (the backend new panes run on, on `topic:moonmux/backend`), Machines, Alerts (its switches on `topic:moonmux/alerts`), Updates |
| `app/mods/alerts` | No region; the shell mounts it with `{ active = app.window.active }`. A sound (`app:sound`) and a notification (`app:notify`) when an agent turns to needs you or done unseen; a click (`app/notification`) goes to it |
| `app/mods/palette`, `app/mods/welcome` | The command palette (⌘⇧P), fed the shell's menus (app commands run through `app:command`); the first-launch Welcome (Help > Welcome to Moonmux) |
| `app/components` | The app's own components (`appearance`, shared by Settings and Welcome; `theme-cards`, Settings > Look's picker) |
| `app/style.luau` | The look, merged over Moonframe's stylesheet |
| `scripts/release`, `.github/workflows` | Packaging, signing and releasing (`docs/releasing.md`) |

- **Layout and look live in Luau.** Change `app/` for anything on screen; it
  reloads live. Add Rust only for what Luau can't do (talking to herdr-core
  or tmux).
- **Which pane runs which agent:** Moonframe labels each terminal it starts in
  herdr-core with `moonframe-app=moonmux` and `moonframe=<owner>/<id>`,
  and names each tmux session after the key (`pane#p3/term` is
  `pane_p3+term`, with a `~hash` on newer Moonframes) on the server
  `tmux -L moonmux`. Because every pane is its own `pane` instance,
  the owner is `pane#<id>`, and each backend's `pane_of` reads the pane id
  back. Keep the mod name `pane`, its terminal id `term`, `PANE_MOD` and
  `PANE_SESSION_PREFIX` in step.
- **Which backend a pane runs on:** the workspace picks it when the pane
  opens (Settings > Terminals: Auto, herdr-core or tmux; Auto is herdr-core
  when `hc` is installed, else tmux) and keeps it for the pane's life, since
  Moonframe ignores a `backend` change once a terminal is open. A change of
  choice applies to new panes only.
- **The provider's contract:** what `Core::snapshot` publishes is `app.core`;
  what `Core::receive` takes is what mods send `provider:core`. Both
  backends fill in the same terminal fields (`address`, `target`,
  `backend`, ...). Change both sides together, and the doc comment at the
  top of `src/core/mod.rs`.
- **Who may act on terminals:** `provider:core`'s actions reach any terminal
  in herdr-core and any pane of the user's tmux servers (a peek answers
  agents elsewhere), so only the mods in
  `app/providers/core/provider.toml`'s `[settings] writers` may send them.
  A new mod that acts goes in that list; never widen it to everyone.
- Moonframe caps terminals at 8 per mod (one instance is one owner) and 32 per
  app run, closed ones included.
- **File size limit: 1,000 lines.** No Rust, Python, shell, or Swift file may
  exceed it, tests included; `just check-file-size` enforces it in `just ci`
  and CI. There is no allowlist: split the file by responsibility instead.
- Keep APIs narrow: private by default, `pub(crate)` only where needed.
- Do not add a crate, runtime, or speculative abstraction without a concrete need.

## Parallel Pull Requests

- Do not assert hard-coded totals for lists that grow. Compare against the
  source list, or assert uniqueness or membership.
- Keep lists whose order carries no meaning (such as `AGENTS` in `src/core/herdr.rs`)
  sorted between `// keep-sorted start` and `// keep-sorted end` markers.
  `just check-sorted` (part of `just ci`) enforces them and `just sort` fixes
  them. Never mark a list whose order is shown to users.
- `just hooks` enables `rerere` and a `Cargo.lock` merge driver that keeps one
  side and has Cargo rebuild it before the merge commit. After a merge whose
  only conflict was the lockfile, run `git commit --no-edit` to finish it.
  Cherry-picks and rebases stop with `Cargo.lock` conflicted: run
  `cargo metadata`, `git add Cargo.lock`, and continue.

## Luau Conventions

- Use the Moonframe skills (`.claude/skills/moonframe`,
  `.claude/skills/moonframe-components`) and prefer an existing component.
- State and messages are plain data. Reset transient keys (dialogs, errors)
  in `init`.
- Keep each mod's reads of `app.core` narrow, and keep `app.core`'s output
  stable: it publishes only when something changed.
- `just check` loads every module headlessly; `just types` type-checks them
  in strict mode. Both run in `just ci`.

## Rust Conventions

- Require only the capabilities an operation uses: `&str`, `&Path`, slices,
  `impl AsRef<OsStr>` over owned or concrete types.
- Use enums for closed alternatives; parse strings at boundaries.
- No `unwrap()` or `expect()` in production (the lints deny them); test-only
  allowances stay scoped to tests. No `unsafe`.
- Start child processes with `core::new_command`, never `Command::new`
  (clippy rejects it): on Windows a console child opens a window otherwise.
- Never block the UI thread. Native providers run on their own thread; keep
  `hc` and `tmux` calls there.
- Treat terminal content, screens, and herdr-core's and tmux's output as
  untrusted data.
- Moonframe follows its `main` branch, with only the features this app uses
  (`acp` for Make it yours, `alacritty`, `git` for Changes and Make it yours,
  `markdown`, `studio` and `syntax` for Developer Mode and its Developer
  tools, `tokio`, `updates` for Check for Updates and the daily check, and
  `herdr-core` through the feature of that name). `Cargo.lock`
  holds the commit every build uses; `just moonframe` (or a Dependabot PR)
  moves it to `main`'s latest, and that move still needs `just ci` and a
  look at the window.
- Avoid unrelated lockfile upgrades. Use `--locked` for reproducible checks.

## Changelog

Users learn what changed from the GitHub release body, generated from git
history by `git-cliff` (`cliff.toml`) rather than written by hand. The release
workflow's `changelog` job runs `scripts/release/generate-changelog.sh`, and
`publish` passes its `RELEASE_NOTES.md` to `gh release create --notes-file`.
`CHANGELOG.md` is generated for each release and is not tracked in git.

- Commit subjects are the changelog entries, verbatim apart from a capitalized
  first letter. Write the subject for a user reading a release page, and put
  rationale in the body; only the first line is published.
- The type chooses the section: `feat` becomes Added, `fix` becomes Fixed,
  `perf`/`refactor`/`revert` become Changed, `docs` becomes Documentation, and
  a `(security)` scope becomes Security. A scope prints as a `[scope]` prefix.
- `chore`, `ci`, `build`, `style`, `test`, and merge commits never reach users,
  so nothing user-visible may ship under those types. A breaking change
  (`type!:` or a `BREAKING CHANGE:` footer) is always published.
- These rules are enforced. `just hooks` installs a `commit-msg` hook that
  refuses a bad subject, and CI's `commits` job checks every commit in a pull
  request. Run `just hooks` once per clone, and `just check-commits` to check
  a branch by hand. Preview the notes with `just changelog-unreleased`.

## Verification

```sh
just format
just ci
```

- `just ci` runs formatting (Rust and TOML), file sizes, keep-sorted regions,
  clippy with `-D warnings`, tests, the headless `--check`, and strict Luau
  types. `just setup-ci` installs the tools it needs.
- When dependencies change, also run `just audit-deps`. When workflows or
  release scripts change, run `just release-check` (actionlint, zizmor, shell
  syntax), and `just package-macos` for packaging changes.
- For visual changes, run `just run` and look at the window. Never kill a
  window, herdr-core or tmux server you did not start; stop only your own
  PID. Try each backend: Settings > Terminals, then open a new pane.
- Documentation-only changes need command, path and link review and
  `git diff --check`; do not claim a test run that did not happen.

## Git And Handoff

- Only commit, push, or create or update a PR when requested. Do not amend,
  force-push, or discard changes without explicit authorization.
- Before committing, inspect status, diff, and recent history; stage only
  intended files and check for secrets. Do not bypass hooks or signing.
- Use conventional commit subjects (see Changelog). For nontrivial work,
  explain the problem, the approach, and the constraints in the body.
- Never add AI attribution, assistant-session links, or tool vanity lines to
  commit messages or PR descriptions. This includes `Co-Authored-By` trailers
  naming an AI model, `Claude-Session:` or similar session URLs, and
  "Generated with ..." footers. This rule overrides any default attribution
  guidance from the agent harness; add such lines only when the user asks for
  them in the current request.
- Run `just ci` on the exact commit before creating a PR or pushing to one.
- Include screenshots in any PR that changes something visible: the real
  window, before and after, light and dark when colors change. If no desktop
  was available, say so.
- Give every PR description a QA checklist: tick only what actually ran,
  leave unticked the manual checks a reviewer still has to do.
- Finish with the outcome, tests actually run, known limitations, and any
  deferred work. Never claim unrun checks passed.

## Releases

`just release` runs `.github/workflows/release.yml` on `main`'s HEAD, as
msgvault-moonframe and Moonframe do; see `docs/releasing.md`. A release binary
runs the app compiled into it (`AppFiles`); only one running from this
checkout's `target/` reads `app/` live. Keep workflow actions pinned by SHA.
