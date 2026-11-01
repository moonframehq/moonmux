<p align="center"><img src="assets/icon.png" width="128" alt="Herdr on Moonframe"></p>

# Herdr on Moonframe

A desktop app for terminals and coding agents, built on
[Moonframe](https://github.com/penso/moonframe): its whole window is laid out
by Luau in `app/`, and its terminals run in
[herdr-core](https://github.com/herdrdev/herdr-core), so they keep running
when the window closes.

![Herdr on Moonframe: three spaces in the sidebar, a tab split into three
herdr-core terminals, and the status line](docs/screenshots/herdr.png)

- **Spaces** hold work in one folder; each has **tabs**, and each tab splits
  into **panes** (right or down), each a live shell. An unnamed space or tab
  follows what runs in it.
- **Agents** in your panes show in the sidebar as Working, Needs you, Done
  or Idle; click one to jump to its pane. Agents running **elsewhere** in
  herdr-core (other apps, `hc run`, workers) are listed too: click to
  **peek** at their screen and answer them (text, Enter, Esc, interrupt).
- **Alerts**: when an agent needs you or finishes while you look
  elsewhere, Herdr plays a sound and posts a notification; click it to go to
  the agent. Settings (⌘,) > Alerts turns them off, or on for agents
  elsewhere too.
- **Settings > Look** picks System, Light or Dark, a theme for each, the tab
  style, the pane headers and the status line; the status line shows
  herdr-core's version and what needs you.

## Install

Download the latest [release](https://github.com/penso/herdr-moonframe/releases):
the macOS disk image (Apple Silicon and Intel), or the Linux or Windows
archive. Herdr needs [herdr-core](https://github.com/herdrdev/herdr-core)
(`hc`) installed; see its README for how.

Every release is signed with Sigstore and carries GitHub attestations; see
[verifying a download](docs/releasing.md#verifying-a-download).

## Keys

| Key | Does |
| --- | --- |
| ⌘N | New space |
| ⌘T / ⌘⇧W | New tab / close tab |
| ⌘D / ⌘⇧D | Split right / down |
| ⌘W | Close pane (ends its shell) |
| ⌘1…⌘8, ⌘9 | Tab N, last tab |
| ⌘⌥← / ⌘⌥→ | Previous / next tab |
| ⌘⌥↑ / ⌘⌥↓ | Previous / next space |
| ⌘B | Show or hide the sidebar |
| ⌘J | Peek at the agent that needs you |

Right-click a space or tab to rename it.

## Build from source

Needs herdr-core, and Rust (see `rust-toolchain.toml`).

```sh
just run      # the window; edits to app/ show live
just check    # load the app headlessly
just hooks    # once per clone: commit-message hook, Cargo.lock merges
just ci       # what CI runs (`just setup-ci` installs its tools)
just bundle   # target/bundle/Herdr.app
```

Releases are cut with `just release`; see [docs/releasing.md](docs/releasing.md).
Contributors and agents: read [AGENTS.md](AGENTS.md).

## License

MIT; see [LICENSE](LICENSE). Third-party icons in the app are credited in
[NOTICE](NOTICE).
