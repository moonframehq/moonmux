<p align="center"><img src="assets/icon.png" width="128" alt="Herdr on Moonframe"></p>

# Herdr on Moonframe

A desktop app for terminals and coding agents. Split your work into spaces,
tabs and panes, see at a glance which agent is working, done or waiting on
you, and close the window without losing a single shell: terminals run in
[herdr-core](https://github.com/herdrdev/herdr-core), or in
[tmux](https://github.com/tmux/tmux) when herdr-core isn't installed, and
are still there when you come back.

Herdr is **malleable software**: built on
[Moonframe](https://github.com/penso/moonframe), its whole window is a few
small [Luau](https://luau.org) scripts you can change while it runs. See
[Make it yours](#make-it-yours).

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
archive.

Herdr keeps its terminals in whichever of these it finds first:

| Installed | Terminals | Agents |
| --- | --- | --- |
| [herdr-core](https://github.com/herdrdev/herdr-core) (`hc`), recommended | Keep running after the window closes | Listed with their status, alerts and peek |
| [tmux](https://github.com/tmux/tmux) | Keep running after the window closes, on Herdr's own tmux server (`tmux -L herdr-moonframe`) | Not tracked |
| Neither | A plain shell that ends with the window | Not tracked |

Install herdr-core (see its README) for everything above; tmux alone gives
you shells that survive the window.

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

## Make it yours

Most desktop apps are finished when you get them: you can change their
settings, and that's it. Herdr is built on
[Moonframe](https://github.com/penso/moonframe), a framework for
**malleable software**, apps their users reshape while they run. Every part
of Herdr's window (the sidebar, the tab strip, the panes, the status line,
the Settings dialog) is a short Luau file, and Moonframe redraws the window
the moment one changes, keeping your spaces, tabs and shells as they were.
No rebuild, no restart.

So if you want a different sidebar, a new status line widget, another
shortcut, or a colour that isn't in any theme, you can have it:

- **Ask an agent.** **Send feedback** (the title bar's `?`) hands what you
  write to a coding agent (Claude, Codex, Gemini and others), which changes
  the app in front of you.
- **Do it yourself.** Put your own files in `~/.config/herdr-moonframe`:
  they are laid over the built-in app, so a mod or a `style.luau` there
  replaces or adds to Herdr's own. Updates never touch that folder.
- **Change Herdr itself.** Clone this repository and `just run`; edits to
  `app/` show live (see below). `app/main.luau` lays out the window,
  `app/mods/` fills it, and `app/style.luau` styles it.

Luau runs sandboxed, with no direct access to your files, network or
processes. A script reaches them only through the operations Herdr's Rust
providers allow it: the Files panel reads the files it shows, and only the
mods listed in `app/providers/core/provider.toml` may type into, read or close
terminals. A script you add can use those operations too, so only add
scripts you trust. Moonframe's
[documentation](https://docs.moonframe.dev) covers mods, components and
styling.

## Build from source

Needs Rust (see `rust-toolchain.toml`), and herdr-core or tmux for lasting
terminals.

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
