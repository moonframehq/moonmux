<p align="center">
  <img src="assets/icon.png" width="160" alt="Moonmux: a terminal prompt on Moonframe's teal badge, in a brown tile">
</p>

<h1 align="center">Moonmux</h1>

<p align="center">
  Terminals and coding agents in one window, kept by herdr-core or tmux.
</p>

<p align="center">
  <a href="https://github.com/penso/moonmux/releases"><b>Download</b></a> ·
  <a href="#install"><b>Install</b></a> ·
  <a href="#gallery">Gallery</a> ·
  <a href="#what-you-get">What you get</a> ·
  <a href="#keys">Keys</a> ·
  <a href="#make-it-yours">Make it yours</a> ·
  <a href="#build-from-source">Build from source</a> ·
  <a href="#license">License</a>
</p>

Moonmux is a desktop app for terminals and coding agents. Keep each repo in a
**space** and each task in a **tab**, split tabs into panes, start Claude Code,
Codex or another agent from the **+** menu, and see at a glance which agent is
working, which needs you and which is done. Its terminals don't live in the
window: they run in [herdr-core](https://github.com/herdrdev/herdr-core) or in
[tmux](https://github.com/tmux/tmux), so closing, updating or crashing the app
leaves every shell and agent running, and the next launch picks them up again.

It is built on [Moonframe](https://github.com/penso/moonframe): the whole
window is Luau in `app/`, drawn by Rust and GPUI, and reshapes itself as the
files change. Ask an agent for a change, from the app's own **Send feedback**
button or from a terminal beside it, and the running window follows.

## Gallery

<p align="center">
  <img src="docs/screenshots/window.png" alt="The Moonmux window: on the left the spaces moonframe, msgvault-moonframe and docs-readme, and below them the agents list with Claude Code idle in moonframe; in the middle a tab split into three panes, Claude Code on the left and two shells on the right with the git history and the folder's files; on the right the Files panel with the moonframe folder; and the status line with herdr-core's version" width="900">
  <br>
  <sub><b>The window.</b> Spaces on the left, with the agents in them below, grouped by what they need from you. Each pane's header names its program and folder; the <b>+</b> menu starts agents, and <b>Files</b> and <b>Changes</b> open the focused pane's folder on the right.</sub>
</p>

<table>
  <tr>
    <td width="50%" valign="top">
      <img src="docs/screenshots/dark.png" alt="The same window in dark mode">
      <br>
      <sub><b>Light and dark.</b> System, Light or Dark, with a theme of your choice for each, in Settings &gt; Look; the terminals follow.</sub>
    </td>
    <td width="50%" valign="top">
      <img src="docs/screenshots/new-menu.png" alt="The + menu open over the tab strip: Terminal with ⌘T, then Start an agent with Claude Code, Codex, OpenCode, Copilot and Gemini numbered 1 to 5, the agents that are not installed greyed out, and New space with ⌘N">
      <br>
      <sub><b>Start an agent.</b> The <b>+</b> menu opens a terminal, or starts Claude Code, Codex, OpenCode, Copilot, Gemini and others in a new tab, by number; those not installed are greyed out.</sub>
    </td>
  </tr>
  <tr>
    <td width="50%" valign="top">
      <img src="docs/screenshots/changes.png" alt="The Changes panel beside a terminal in the docs-readme worktree: README.md changed, 111 lines added and 31 removed, and a commit message field with a Commit button">
      <br>
      <sub><b>Changes.</b> What changed in the focused pane's repository, with each file's diff a click away, and a commit box. <b>Files</b> browses the same folder with read-only previews.</sub>
    </td>
    <td width="50%" valign="top">
      <img src="docs/screenshots/settings.png" alt="The Settings dialog on Look: Appearance System, Light or Dark, and rows of theme cards for dark and for light, each with its colours">
      <br>
      <sub><b>Settings.</b> Look (appearance, themes, tabs, pane headers), Terminals (herdr-core or tmux), Machines, Alerts, Updates and Privacy. <b>⌘⇧P</b> finds any command.</sub>
    </td>
  </tr>
</table>

## What you get

- **Spaces, tabs and panes.** A space holds work in one folder, usually a
  repo; it has tabs, and each tab splits right or down into panes, each a
  live shell. An unnamed space or tab follows what runs in it.
- **Terminals that outlive the window.** Panes run in herdr-core or tmux,
  your choice in Settings > Terminals: Auto takes herdr-core when it is
  installed, else tmux. A pane stays on the one it opened on, through
  relaunches, and its header names it. With tmux, Moonmux's terminals live on
  a server of their own: `tmux -L moonmux ls`.
- **Agents in a click.** The **+** menu starts Claude Code, Codex, OpenCode,
  Copilot, Gemini, Pi, Cursor and more in a new tab, named after the agent
  and drawn with its icon.
- **Who needs you.** The agents list groups agents by Needs you, Working,
  Done and Idle, with how long each has been so, and shows the helpers an
  agent started under it. A click shows the agent's pane; **⌘J** goes to the
  one that needs you.
- **Alerts.** When an agent needs you or finishes while you look elsewhere,
  Moonmux plays a sound and posts a notification; click it to go there.
- **Peek.** Agents running elsewhere (another app, `hc run`, your own tmux
  sessions) are listed too: peek at their screen and answer them without
  leaving your tab.
- **Files and Changes.** The focused pane's folder as a file tree with
  previews, and its git changes with diffs and a commit box.
- **Settings, Welcome and a command palette.** Appearance and themes, tab
  style and pane headers, where terminals run, alerts; a first-run tour; and
  **⌘⇧P** for every command.

## Install

Download the latest [release](https://github.com/penso/moonmux/releases):
the macOS disk image (Apple Silicon and Intel), or the Linux or Windows
archive. Moonmux needs [herdr-core](https://github.com/herdrdev/herdr-core)
(`hc`; see its README for how) or tmux (`brew install tmux`,
`apt install tmux`).

Every release is signed with Sigstore and carries GitHub attestations; the
macOS app is notarized. See
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
| ⌘⇧P | Command palette |
| ⌘, | Settings |

Right-click a space or tab to rename it.

## Make it yours

Everything on screen is a Luau file in `app/`: the shell in `app/main.luau`,
each panel a mod in `app/mods/`, the look in `app/style.luau`. Copy one into
`~/.config/moonmux` (same layout) and change it there: the window reloads as
you save, and updates never touch your copy. Agents learn how from
Moonframe's skills in `.claude/skills`, and **Send feedback** hands your
request to one, which changes the app in front of you. See
[Moonframe](https://github.com/penso/moonframe) for the building blocks.

## Build from source

Needs herdr-core or tmux, and Rust (see `rust-toolchain.toml`). Each
backend is a cargo feature, both on by default: build one alone with
`cargo build --no-default-features --features tmux` (or `herdr-core`).

```sh
just run      # the window; edits to app/ show live
just check    # load the app headlessly
just hooks    # once per clone: commit-message hook, Cargo.lock merges
just features # clippy and tests with each backend alone
just ci       # what CI runs (`just setup-ci` installs its tools)
just bundle   # target/bundle/Moonmux.app
```

Releases are cut with `just release`; see [docs/releasing.md](docs/releasing.md).
Contributors and agents: read [AGENTS.md](AGENTS.md).

## License

MIT; see [LICENSE](LICENSE). Third-party icons in the app are credited in
[NOTICE](NOTICE).
