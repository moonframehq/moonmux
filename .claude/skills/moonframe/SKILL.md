---
name: moonframe
description: Build anything in Moonframe, a starter app whose window is laid out entirely by Luau. Covers panels, side panels, splits, toolbars, status bars, popovers, overlays (dialogs, command palettes, sheets, toasts) and their keys, mods and extensions, reusable components (`use`), live terminals (`ui.terminal`, kept by herdr-core or tmux across launches), state kept across launches (saved on its own, in herdr-core or SQLite), providers (Wasm, process, or native Rust built into the app, such as git, which also commits and pushes for a mod and answers it), icons, and styling through style.luau (colors, fonts, terminal fonts and Nerd Font icons, corners, shadows, spacing, animations). Also covers acting on the user's Inspect Element selection ("add X here"). Use whenever the task is to add, move, restyle or experiment with any UI in moonframe, to build a widget, component, provider or extension, or to customise the installed app in ~/.config/moonframe. The window reloads live as files change, so no Rust build is needed for Luau work.
---

# Moonframe

Luau describes every part of the window. Rust only checks the trees the
scripts return and draws them with GPUI. To change the UI, edit Luau under
`app/`, never Rust, unless the task needs a new element type (see "Extending
the host" below).

```text
app/
  main.luau              the shell: lays out the whole window
  style.luau             the stylesheet: colors, corners, fonts, rules (optional)
  mods/<name>/main.luau  a mod; the shell places it by region or by name
  components/<name>.luau a ready-made piece of view mods `use` (see below)
  providers/<name>/      a provider: data Luau can't produce (see below)
```

**Two folders, laid over each other.** The window loads the built-in app
(`app/` in this repository; `Contents/Resources/app` in an installed
bundle), then the user's folder, `~/.config/moonframe` (or
`$XDG_CONFIG_HOME/moonframe`), on top. Both have the layout above.
- A mod or provider in the user's folder replaces the built-in one of the
  same name, whole. A new name adds one.
- A user `main.luau` replaces the shell.
- A user `style.luau` is merged over the built-in one: each color and speed
  replaces the one of its name, each `mono` key replaces the app's
  (`mono = { size = 14 }` keeps its family), each rule's properties replace
  those of the same rule (`panel = { radius = 4 }` keeps the app's panel
  background), and any other key replaces the app's.
- Where to write: when working on this repository, edit `app/`. When the
  user wants to customise the app they use (or it is installed), write to
  `~/.config/moonframe` and never into the bundle, which is signed and
  replaced by each update. To change one built-in mod, copy its folder there
  and edit the copy.
- The window watches both folders, so either kind of edit reloads live.
- **Packages** go between the two: mods others publish as git repositories,
  installed with `moonframe mod install` (see "Sharing mods"). A package
  never replaces the shell, and the user's folder goes over packages.

Layout and look are separate. Modules decide *what* goes *where*;
`style.luau` decides how it looks. Prefer a stylesheet rule or class over
repeating `style = { ... }` on many elements.

## Building something: where it goes

Pick the lowest layer that can do the job:

| The task needs | Build | Where | Rust build? |
| --- | --- | --- | --- |
| something on screen, from data Luau already has | a **mod** (or edit one) | `mods/<name>/main.luau` | no |
| the same piece of UI in several places | a **component** | `components/<name>.luau` | no |
| a shell or command line in the window | `ui.terminal` or `use "terminal"` | in a mod | no |
| tabs the user opens and closes, remembered | `use "tabbed"`, and a `tab` entry on what they show | in a mod | no |
| a widget with memory of its own, reusable | a component with `update` (an instance) | `components/<name>.luau` | no |
| several modules sharing a value | a topic: publish to `topic:<name>`, `subscribe` to it | in the mods | no |
| a web page in the window | `ui.browser` | in a mod | no |
| a text field, search box, prompt or text area | `ui.input` or `use "search-field"` | in a mod | no |
| data from the machine, files, network or a CLI | a **provider** feeding `app.<name>` | `providers/<name>/` | no (the host builds Wasm providers itself) |
| state that survives quitting the app (open panels, tabs, a choice) | nothing to add: every mod's state is saved; shape what comes back in `init` | the mod's `init` | no |
| git status, a file's diff, commit or push | read `app.git`; send `provider:git` a message | in a mod | no (git is built in) |
| markdown shown, or a project file read and saved | `ui.markdown` or `use "markdown"`; `use "markdown-editor"`, which reads and saves through `provider:docs` | in a mod | no (docs is built in) |
| a coding agent doing a task (Claude, Codex, Gemini...) | send `provider:acp` `{ start = ... }`; show the run with `use "agent-run"` | in a mod | no (acp is built in) |
| a provider that needs a Rust library or answers actions | a **native provider** | `crates/` + `providers/<name>/provider.toml` | yes, and the app restarts |
| a look change | a rule in `style.luau`, or the mod's `styles` | | no |
| a new kind of element (tree, chart, image) | a host element | `crates/` | yes, and the app restarts |

Prefer what already exists. Check `app/components/` before writing a widget,
and the element table before asking for a host change. Live-reloaded Luau
keeps the user's window running; Rust changes don't.

### Recipes

**"Show me some ideas first."** When the user wants options or mockups
before a change, use the **moonframe-mockups** skill
(`.claude/skills/moonframe-mockups`): variants drawn live in a throwaway
`mods/mockup` in the user's folder, opened in a tab, applied only once the
user picks one.

**"Add X here."** Read `~/.cache/moonframe/selection.md`
(see "Here: the user's selection"). Open the file at the line it gives,
use a component if one fits, `just check`, and tell the user what you added
and where.

**A new panel in a side area.** Create `mods/<name>/main.luau` with
`region = "left"` (or `right`, `main`, `bottom`, `status`, `status-right`)
and a `view` returning `ui.panel { title = ..., grow = true, ... }`. It
appears without touching the shell.

**A status bar widget.** A mod with `region = "status-right"` (or
`status`) and `order` to place it. The view is a small `ui.row { align =
"center", gap = "small", ... }`, for example from `use "stat"`. Give
changing numbers a fixed width.

**A widget with live data.** Read `app.<provider>`, and draw a placeholder
while it is nil. If no provider has the data, write one (see "Providers").
Never invent the data in Luau unless it is labelled demo data.

**Something clickable that opens a sheet.** `ui.popover` around the
widget, with `open` in the mod's state (see "Sheets and menus").

**A dialog, command palette, side sheet or toast.** `ui.overlay` with
`open` in the mod's state, a `place`, and `keys` for Escape, Enter and the
arrows (see "Dialogs, palettes, sheets and toasts").

**A reusable piece.** A component with a good `example`, so it previews
in the Components window, then `use` it from the mod.

**Markdown.** To show it: `local markdown = use "markdown"`, then
`markdown { text = state.notes, grow = true, scroll = true, link =
"open-link" }` (or `ui.markdown`); a link click sends `{ msg = "open-link",
url = ... }` and opens nothing by itself, and pictures are not loaded. To
edit a project file: `local markdown_editor = use "markdown-editor"`, then
`markdown_editor { id = "readme", path = "README.md" }` in a growing
container; it reads and saves through the `docs` provider (see "Built-in
providers"), which takes saves only from its `writers` (`editor` as
shipped; add yours there as for git). In the starter, a markdown file
picked in the explorer opens in a main tab of this kind.

**A web page.** With browser chrome (back, forward, reload, an address
bar that navigates, the title): `local browser = use "browser"`, then
`browser { id = "docs", url = state.url, page = state.page, navigate =
{ navigated = true } }` and `state.page = msg` in `update`
(`app/components/demos/browser.luau` is the whole mod). A bare page:
`ui.browser { id = "docs", url = "https://example.com" }` inside a
container with `grow = true`, with buttons sending
`command = "browser-back", browser = "docs"` (see "Browsers").

**A terminal.** `local terminal = use "terminal"` and
`terminal { id = "main", cwd = "~/code" }` inside a container with
`grow = true` (see "Terminals").

**A search field.** Keep the query in state, pass it back as `value`, and
filter on it (`use "search-field"` wraps this; `mods/explorer` uses it):

```lua
update = function(state, msg)
	if msg.msg == "query" then state.query = msg.value          -- each edit
	elseif msg.msg == "clear" then state.query = ""              -- Esc empties it
	elseif msg.msg == "open" then return state, { { to = "editor", msg = { open = msg.value } } }
	end
end,
view = function(state)
	return ui.column {
		ui.input { id = "query", value = state.query, placeholder = "Search…",
			change = "query", submit = "open", keys = { escape = "clear" } },
		-- ...the rows that match state.query
	}
end,
```

For a palette, add `keys = { up = "prev", down = "next" }` and
`focus = state.open`.

**Mods that talk.** Send `msg = ..., to = "other"` from a control, or
return sends from `update`. Messages to `provider:<name>` go to a provider.

**Remember something across launches.** Every mod's state is saved on its
own and comes back to `init` as `previous` next launch (see "State that
outlives the app"). Keep from `previous` only what the user would expect
back (open panels, the tab they were on), and reset transient keys
(`loading`, `hover`, errors, an action still running) in `init`.

**Ask a provider to act, and show the answer.** Native providers answer
actions in the asking mod's `update` (a `Reply`, see "Built-in providers"):

```lua
-- in update
if msg == "commit" and not state.running then
	state.running = "commit"
	return state, { { to = "provider:git", msg = { commit = state.message, id = "mine" } } }
elseif type(msg) == "table" and msg.reply == "mine" then
	state.running = nil                                   -- show msg.message or msg.error
	state.result = { ok = msg.ok, text = msg.message or msg.error }
end
```

Show a spinner while `running` (a push may wait on the user's hardware
key), disable the button meanwhile, and show `error` as the user's next
step. git only takes writes from mods in `providers/git`'s
`[settings] writers` (as shipped, `status`). For a new mod, copy
`app/providers/git/` to `~/.config/moonframe/providers/git/` and add its
name to `writers` there (in this repository, edit `app/providers/git`
itself); otherwise it is refused with a reply saying so. `mods/status` is the full example (a commit sheet on the branch).

**Have a coding agent do something.** The `acp` provider runs Claude,
Codex, Gemini, OpenCode or Copilot over the Agent Client Protocol in the
app's repository and publishes each run as `app.acp.runs`; draw one with
`use "agent-run"` (status, plan, tool calls, its permission question, its
answer), whose buttons talk to the provider directly:

```lua
-- in update (the mod must be in providers/acp's `starters`)
return state, { { to = "provider:acp", msg = {
	start = state.text, id = "mine", agent = "claude",
	instructions = "How to do it, from the mod.",   -- before the user's words
	context = { main_tab = state.tab },             -- fenced as data, never instructions
	thread = previous.id,                           -- optional: a follow-up, shown with the run it continues
} } }
-- answered as { reply = "mine", action = "start", ok, run | error },
-- and when it ends { reply = "mine", action = "finished", ok, run, status, message | error }
-- in view
for _, run in (app.acp and app.acp.runs) or {} do
	if run.owner == "mine" then ... agent_run { run = run } ... end
end
```

Every tool the agent wants waits on the user in `agent-run`, unless the
user set the mod in `providers/acp`'s `[settings] allow`: `"read"` lets
tools that only look go without asking, `"all"` every tool, each allowed
once, never always. A mod can't ask for more; leave that setting to the
user (as shipped, `feedback = "read"`). Like git's
`writers`, a new mod goes in `starters` of a copy of `app/providers/acp`.

**Customising the installed app.** Write to `~/.config/moonframe`
(same layout as `app/`). Copy a built-in mod's folder there to change it.
Personal mods belong there, never in this repository. `just install` puts
the current build in `/Applications` so you can try them against it.

Then always: `just check`, `just types`, and if the user's window is open,
it has already reloaded.

## "Here": the user's selection

The user picks elements in the window with **View > Inspect Element**
(⌥⌘I), like a browser's developer tools. The pick is saved to
`~/.cache/moonframe/selection.md` (`$XDG_CACHE_HOME` if set), and the
user can copy the same text with **Copy for agent**:

```text
Selected in Moonframe (View > Inspect Element):
- element: text.status-text "⎇ main +29"
- mod: status, drawn by /…/app/mods/status/main.luau:33
- inside: mod shell > column > … > region "status-right" > mod status > row > text.status-text "⎇ main +29"
```

When the user says "here", "this", "the selected element" or "add X there"
without saying where, read that file first. The `file:line` is the `ui` call
that built the element. For an element a component built, it is the line in
the mod that used the component. Change or add next to that line. "Inside"
lists every element around it, outermost first, so "add a list here" on a
row means adding a child to that row; on a text, add a sibling. If the file
is missing, nothing is selected: ask, or pick a sensible place and say which.

**A mod can ask for a pick.** A button with `command = "inspect"` starts
picking for the mod that drew it, without the card. That mod hears it on the
topic `app/inspect` (subscribe to it): `{ asker, picking = true }` as it
starts (hide a dialog then), and `{ asker, picking = false, picked, element?,
mod?, at?, report? }` when the user clicks an element (`picked = true`;
`report` is the text above) or presses Escape. Only the mod that asked
hears it; if another mod starts a pick meanwhile, the first hears
`picked = false`. `mods/feedback` attaches the pick to a request this way.
`app/` topics are the app's: a mod can't publish them, nor send another mod
a message passing for `app/` or `provider/` topic news.

"Copy for agent" copies that text and closes the card, but the file stays,
so the selection is still there for you. "Clear" (or Esc) deletes it. The
Components window's "Copy for agent" copies a request to add that component
at the current selection, with its usage. Treat either as the user's
instruction.

## Components: ready-made pieces

`components/<name>.luau` (in `app/` or `~/.config/moonframe`, which
wins by name) holds a component: a function from props to elements, plus
what the Components menu shows. The user browses them in their own window
with **Components > Browse Components…** (⇧⌘K), or picks one by category
from that menu. Each gets its icon, a live preview of `example`, and its
usage; **Copy for agent** asks you to add it at the selection. Adding or
editing a component file updates the menu and the window live.

Use one in a mod with `use`, and call it wherever an element goes:

```lua
local stat = use "stat"
local list = use "list"

return mod {
	view = function(state, app)
		return ui.column {
			stat { icon = "cpu", label = "CPU", value = "12%", width = 32 },
			list { rows = state.rows, selected = state.selected },
		}
	end,
}
```

Before writing a widget, read the **moonframe-components** skill
(`.claude/skills/moonframe-components/SKILL.md`): the full catalogue of host
elements and components, with when to use each, their props, the classes
they style, and how to call them. `just check` lists the names too. After
adding or changing a component, run `just components` to regenerate that
catalogue (`just ci` fails when it is stale).

A new component looks like this:

```lua
return component {
	title = "Stat",                 -- the menu and gallery name
	category = "Status bar",        -- the Components submenu
	icon = "cpu",                   -- beside its title in the browser
	description = "One line on what it is for.",
	-- For the catalogue: when to use it, and what to use instead.
	when = "One labelled value in a status bar. For a fraction use `meter`.",
	props = { value = "the text shown" },
	-- Default rules for its classes. They apply wherever it is used, under
	-- the mod's own styles and style.luau, which can restyle it.
	styles = {
		[".stat"] = { padding = { 2, 6 }, radius = 6 },
		[".stat:hover"] = { background = "accent/10" },
	},
	example = { value = "12%" },    -- props for the preview
	-- usage = [[...]],             -- Luau to show, when `example` holds elements
	build = function(props)
		return ui.row { class = "stat", ui.text { props.value } }
	end,
}
```

- `build` runs in the sandbox of the mod that uses it and returns elements
  like a `view`. Clicks it builds go to that mod's `update`.
- Without `usage`, the gallery shows `local x = use "name"` and a call
  with `example`. Give `usage` when the example has elements or children.
- Write components to work anywhere: their look goes in their own
  `styles` under classes named after them (`.tabs`, `.tabs-tab`,
  `.tabs-tab:selected`), in theme colors, so `style.luau` can restyle every
  part. Avoid inline `style` except for values computed from props (a
  meter's width). Nothing a mod must set up.
- Always give `when` and `props`; `just components` refuses a component
  without `when`.
- The preview is drawn from `example`, and a broken component shows its
  error in the gallery and in `just check`. Previews never start anything:
  a terminal shows a placeholder.
- A component can `use` another; one that uses itself is an error.
- **Demos.** The browser's preview is live: it runs a demo, a whole mod
  that uses the component, and shows that file as the code to copy. Write
  `components/demos/<name>.luau` (`return mod { init, update, view }`) for
  any component with clicks or state, so the user can play with it: a
  segmented control that switches, a dialog a button opens, a tree that
  folds. Keep it short, realistic and commented. Without one, the preview
  is a mod made from `example`, unless the component gives `usage`: then
  the preview is a still picture whose controls do nothing, so a component
  with `usage` and clicks needs a demo file. A demo runs in the gallery
  only: its messages reach only its own `update`, app commands do nothing,
  and its text fields edit (outside a demo they only show their text).
- Name it after what it is (`branch-list`, not `widget2`); the file name
  is the name `use` takes.
- Types: the definition file's type names (`IconName`, `Tone`, `Style`)
  are not visible to scripts, so annotate props with `string` or `any`,
  and cast where a `ui` field wants a union (`name = props.icon :: any`).

## Terminals

`ui.terminal` (or the `terminal` and `ghostty-terminal` components) shows a
live shell. The host runs it, so it can't be written in Luau:

- **What runs:** always the user's login shell. A mod chooses `cwd`, never
  a command, so the sandbox still can't run programs on its own.
- **Lifetime:** the host keeps one shell per mod and `id`. It survives
  reloads and hidden panes; with herdr-core or tmux it survives quitting
  the app too, and the next launch attaches to it again. It ends when the
  shell exits (Enter starts another). Two `id`s are two shells.
- **Backends:** `backend = "auto"` (default) runs it on herdr-core (`hc`)
  when installed (starting its server if needed), else tmux (a session per
  terminal on the app's own server: `tmux -L moonframe ls`, `tmux -L
  moonframe-<profile> ls` for a profile, `tmux -L <name> ls` for an app on
  the library; herdr-core labels each app's apart), else a local
  pty that ends with the app. `"herdr-core"`, `"tmux"` or `"pty"` asks for
  one; one that is missing falls back to the next. A faint label in the
  terminal's bottom-right corner names the backend in use.
- **Engines:** `engine = "alacritty"` (default, pure Rust) or `"ghostty"`
  (libghostty-vt). Both are cargo features of the `moonframe` crate; the
  starter builds both, an app on the library opts into `ghostty`.
  A build without one shows why in the element's place.
- **Sizing:** it grows to fill its container (give the container `grow`),
  and the shell is resized to match. Style it with a `terminal` rule or
  `style`: `background` and `color` set its default colors.
- **Font:** the stylesheet's `mono` sets it for every terminal: family,
  size, weight, line height, fallbacks, ligatures (see "Fonts" under
  "Stylesheet"). Icons showing as boxes in vim, starship or tmux status
  lines mean a missing Nerd Font; fix it there, not in the mod.
- **Input:** click to focus. While it has focus every key goes to the
  shell before the app's shortcuts: Tab (completion), Esc, Ctrl keys,
  arrows. ⌘V pastes (control characters are dropped from pastes, so they
  can't escape bracketed paste); other ⌘ shortcuts stay the app's. The
  scroll wheel moves through the scrollback.
- **Limits:** `id` is 1 to 64 of `a-z A-Z 0-9 - _`. A mod can have 8
  terminals and the app 32; past that the element shows why. Don't make
  ids from data (one per list row): each is a process.
- **In the component browser** a terminal shows a placeholder and starts
  no shell.

## Browsers

`ui.browser` shows a web page in a native web view the host runs (WebKit
on macOS, WebView2 on Windows). A mod gives it an address and reads back
where it went; it can't run scripts in the page.

```lua
ui.browser {
	id = "docs",                      -- one page per mod and id
	url = state.url,                  -- http, https or file
	navigate = { navigated = true },  -- arrives as { navigated = true, url, title, loading, back, forward }
	loading = "loading",              -- arrives as { msg = "loading", loading = true|false, url = ... }
	zoom = 1.25,                      -- 0.25 to 5
}
ui.button { "Back", command = "browser-back", browser = "docs" }
ui.input { id = "docs-address", value = state.page.url, browser = "docs" }  -- its address bar
```

- **Lifetime:** the host keeps one page per mod and `id`, like a terminal.
  It survives reloads and hidden panes (hidden, it keeps its place and
  history) and ends with the app.
- **`url`:** the page goes there when the mod passes an address different
  from the last one it passed. Links the user follows stand until then.
  To load the same address again, use `browser-reload`. Only `http://`,
  `https://` and `file://` addresses are read; anything else is an error
  that fails the mod's whole view, so never pass text the user typed as
  `url`: make the field an address bar (below), which also loads the
  address already shown, which a `url` the mod sets again cannot.
  The page itself may follow links only to those (and `about:blank`); a
  link that opens a new window opens in the same page, and downloads are
  refused.
- **Address bar:** `ui.input { browser = "<id>" }` (with `to = "mod"` for
  another mod's) drives the page itself, with nothing in `update`: Enter
  loads the text, even the address already shown (`example.com` over
  https, `localhost:3000` over http, words as a DuckDuckGo search), and
  hands the keyboard back; focus selects the whole text; Escape puts back
  the page's address. Pass the page's address (from `navigate`) as its
  `value` so it follows the page.
- **Messages:** `navigate` is sent as the page goes somewhere, its title
  changes, or a load starts or ends, with `url`, `title`, `loading`, and
  `back` and `forward` (whether it can go there; macOS only, absent
  elsewhere): keep the whole message in state. `loading` is sent as a load
  starts and finishes. A table message gets
  the fields added; any other value arrives as `{ msg = <value>, ... }`.
  Both go to the drawing mod's `update`.
- **Chrome:** `command = "browser-back"`, `"browser-forward"`,
  `"browser-reload"`, `"browser-stop"` or `"browser-open"` (the page's
  address in the system browser) on a button, item, row or column,
  with `browser = "<id>"`, runs on that browser of the drawing mod (`to =
  "mod"` for another mod's). The bar around it is Luau: the `browser`
  component is one, with back and forward greyed out at the ends of the
  history and reload turning into stop while the page loads
  (`app/components/demos/browser.luau` is a mod using it).
- **Sizing and look:** it fills its container (give it `grow`). The box is
  styled by the `browser` rule, a class or `style` (`background`, `border`,
  `padding`, `radius`, ...); the page fills the inside, square-cornered.
- **Over the page:** the page is a native view above the window's drawing,
  so nothing the window draws can lie over it. While a popover, the toast,
  or the inspector's picker or card reaches it, the page hides. Tooltips
  under the pointer don't hide it.
- **Input:** click the page to give it the keyboard; a click anywhere else
  in the window takes it back. While it has it, every key goes to the page
  (on macOS ⌘X, ⌘C, ⌘V, ⌘A, ⌘Z and ⇧⌘Z are its edit actions), except
  ⌘Q, ⌘H, ⌥⌘H, ⌘M and ⌘`` ` ``, which stay the app's. A focused terminal
  never sees them.
- **Limits:** `id` is 1 to 64 of `a-z A-Z 0-9 - _`. A mod can have 4
  browsers and the app 16; past that the element shows why.
- **Linux:** wry has no working GPUI child-view backend there yet, so the
  element shows its address with an **Open in browser** button that opens
  it in the system browser.
- **In the component browser** it shows its address and loads nothing.

## Text input

`ui.input` is a text field: a search box, a palette's query, a rename
prompt, a settings value, a commit message (`multiline`). The host does the
editing (caret, selection, IME, undo, copy and paste) the same way on
macOS, Linux and Windows. Its look comes only from the stylesheet.

```lua
ui.input {
	id = "query",              -- keeps the text; default: its place in the tree
	value = state.query,       -- applied only when it changes (see below)
	placeholder = "Search…",
	change = "query",          -- sent on each edit
	submit = "run",            -- sent on Enter (⌘/Ctrl-Enter when multiline)
	keys = { up = "prev", down = "next", escape = "close" },
	focus = true,              -- take focus when it appears or turns true
	-- browser = "docs",       -- or: be that ui.browser's address bar
}
```

- **The text is the host's.** It keeps each field's text by the drawing
  mod and `id` (else the field's place in the tree, which moves when the
  tree around it changes, so give fields that matter an `id`). It survives
  re-renders and reloads. `value` replaces the text only when it differs
  from the last `value` the mod passed, so a mod echoing the text back
  changes nothing, and `value = ""` after a submit clears it. Without
  `value`, the user's text stands.
- **Messages carry the text.** A table `msg` arrives as a copy with
  `value = <text>` added: `change = { query = true }` sends
  `{ query = true, value = "ab" }`. Anything else arrives as
  `{ msg = <msg>, value = <text> }`: `change = "query"` sends
  `{ msg = "query", value = "ab" }`. `to = "mod"` sends them to another mod.
  The host's own `value` changes don't send `change`.
- **`keys`** maps key names to messages. While the field has focus, those
  keys send their message (shaped the same way) instead of editing, ahead
  of the app's shortcuts and any terminal. Names are GPUI's keystrokes:
  modifiers `ctrl` `alt` `shift` `cmd` `secondary` joined by `-`, then a
  lowercase letter, digit or mark (`/`, `-`) or one of `up` `down` `left`
  `right` `escape` `tab` `enter` `backspace` `delete` `insert` `home` `end`
  `pageup` `pagedown` `space` `f1`…`f12`: `["shift-tab"]`, `["ctrl-n"]`,
  `["secondary-k"]`. `secondary` is ⌘ on macOS and Ctrl elsewhere; use it
  for shortcuts that should work everywhere. Modifiers must match exactly
  (`up` is not `shift-up`). A key in `keys` wins over `submit`. Up to 32.
- **`browser = "<id>"`** makes the field that `ui.browser`'s address bar,
  with no messages to handle: Enter loads the text, focus selects it, and
  Escape puts back the page's address (see "Browsers").
- **Focus.** Clicking focuses a field; so does `focus = true`, once, when
  the field appears or `focus` turns from false to true (a field gone for a
  render and back counts as appearing). Previews in the component browser
  never take focus or edit.
- **`multiline = true`** makes a text area: Enter adds a line, ⌘/Ctrl-Enter
  sends `submit`, and `rows` (default 3, up to 200) sets the visible
  height; it scrolls past that. With `grow = true` in a column it fills
  the height it gets instead (an editor's pane), never under `rows`. Its
  `value` may be a whole document, up to 1 MiB.
- **Size.** `width` in pixels, or `grow = true` to take the free space
  along the parent's axis (in a row, the width). In a column it fills the
  width anyway. `mono = true` uses the monospace font.
- **Look.** Style the `input` rule and its states, `:hover`, `:focus` and
  `:disabled`, and `input-placeholder`, `input-selection` and
  `input-caret` (see "Stylesheet"). Without a rule it has a 1px border and
  `{ 4, 8 }` padding, and the caret and selection follow `accent`.
- **No focus look.** The blinking caret is the only sign of focus, by the
  user's choice: don't add an `input:focus` border, ring or glow, and in
  Rust build `gpui_kit` `Input`s with `.focus_bordered(false)` (as the
  components and theme picker searches do). A host window with a search
  field focuses it when it opens (`search.update(cx, |s, cx| s.focus(window, cx))`).

Building the `ghostty` engine needs Zig 0.15 (in `mise.toml`). On macOS 27
and later, Zig 0.15 can't link natively, so `tools/zig/zig` (first on
`PATH` through mise and the justfile) works around it. That needs a macOS
SDK of version 15 or older installed. Build without it with
`--no-default-features --features alacritty`.

## State that outlives the app

Every mod's state, and every instance's, is saved to SQLite whenever it
changes and handed back to `init` as `previous` at the next launch. So state
must stay plain data (see the rules above): a function in it is not saved.

- Where: in herdr-core when it runs here (`hc`, beside the terminals it
  keeps), as the document `moonframe` in the app's own format
  (`crates/moonframe/src/persist.rs`); else
  `~/.local/state/moonframe/state.sqlite` (`$XDG_STATE_HOME`), outside the
  watched config folder. A key herdr-core lacks is read from SQLite, so
  state moves over by itself. With `--no-user` nothing is restored or
  saved, so tests and screenshots start clean.
- Keys: `mod:<name>` for a mod, `mod:<instance key>` for an instance, and
  each topic by name.
- The schema grows by numbered migrations in `crates/moonframe-store`
  (`MIGRATIONS`), recorded in SQLite's `user_version`: only ever append a
  step. A file from a newer app opens read-only.
- To start a mod over, change what `init` builds; to drop old state, ignore
  `previous` when it lacks what you need.

## Instances: mods and components with state of their own

**A mod mounted with an id** is an instance with its own state:
`ui.mount { "explorer", id = "files-2", props = { root = "app" } }`. Its key
is `explorer#files-2`, the same wherever it is mounted. It runs the mod's
file in a Luau state of its own; `init`, `update` and `view` get `props`
last. Without an id, `ui.mount "explorer"` draws the mod itself, as before.

**A component with `update`** keeps state: `use "tabbed"` then returns a
function that mounts an instance, `tabbed { id = "main", ... }`, keyed by
the module that drew it (`editor/tabbed#main`), so two mods' instances are
separate. Its definition:

```lua
return component {
	title = "Counter",
	example = { id = "example" },
	init = function(previous, props) return previous or { count = 0 } end,
	update = function(state, msg, props)
		if msg == "add" then
			state.count += 1
			return state, { { to = "parent", msg = { counted = state.count } } }
		end
	end,
	build = function(props, state, app)   -- props first, then its state
		return ui.button { `{state.count}`, msg = "add" }
	end,
}
```

- Clicks inside an instance go to its own `update`.
- `to = "parent"` reaches the module that drew it; that module reaches it
  as `to = "<component>#<id>"` (`"tabbed#main"`).
- An instance not drawn (a hidden tab) is dropped from memory; its saved
  state brings it back where it was when it is drawn again.
- Ids are plain (`a-z A-Z 0-9 - _`); at most 256 instances are drawn at
  once, so never make an id per row of data.

## Topics: pushing changes to whoever listens

A module publishes with a send: `{ to = "topic:tabs/main", msg = value }`.
The value is kept (and saved), and every module that lists the topic in
`subscribe` hears it in `update` as `{ topic = "tabs/main", value = ... }`:

```lua
return mod {
	subscribe = { "tabs/*", "provider/git" },   -- names, or prefixes ending in *
	update = function(state, msg)
		if type(msg) == "table" and msg.topic == "tabs/main" then
			state.tab_count = msg.value.count
		end
	end,
	view = function(state) return ui.text(`{state.tab_count or 0} tabs`) end,
}
```

- A module that loads or reloads hears each subscribed topic's current
  value at once, so it starts in step; instances too, when mounted.
- `provider/<name>` carries a provider's data each time it publishes, for a
  mod that must react (not only redraw); views still read `app.<name>`.
- Names are 1 to 128 of `a-z A-Z 0-9 - _ . /`; `provider/` is the
  providers', and `app/` the app's own (`app/inspect`, `app/notification`). Previews (the Components window) hear nothing.
- Use a topic when several modules share a value; use `to = "<mod>"` when
  one module tells another.

## Asking the app

A module asks the app itself for something by sending to `app:<what>`,
from `update` (or `init`) as a second result, or from a control's `msg`
with `to`. No click is needed, so a command palette's Enter, a keyboard
shortcut in `keys` or a provider's news can run what a button would.

```lua
update = function(state, msg)
	if msg == "run" then
		local row = state.rows[state.selected]
		return state, { { to = "app:command", msg = row.command } }   -- "minimize"
	end
end,
```

- `app:command`, `msg` a command's name: runs it as a control's `command`
  does, with the same commands (`quit` `reload` `toggle-theme`
  `show-themes` `minimize` `maximize` `close-window` `check-for-updates`
  `inspect`) and nothing else. It runs once the update that asked is done.
- `app:appearance`, `msg` `"system"`, `"light"` or `"dark"`: whether the
  window follows the system's light or dark (and switches live as it
  does) or stays one. It is kept across launches for the app and profile
  (`settings.toml`, as the theme), and changing it keeps the themes picked.
  Views read it as `app.appearance = { mode, dark }` (`dark` is what the
  window is now, as `app.dark`). `toggle-theme` sets the other of light and
  dark; the View menu's Appearance submenu offers all three.

```lua
-- in view(state, app): System / Light / Dark
local buttons = {}
for _, mode in { "system", "light", "dark" } do
	table.insert(buttons, ui.button { mode, to = "app:appearance", msg = mode,
		variant = if app.appearance.mode == mode then "primary" else "ghost" })
end
return ui.row { table.unpack(buttons) }
```

- `app:theme`, `msg` `{ light = "<theme>", dark = "<theme>" }` (either or
  both): the theme light windows show and the one dark windows show, each
  kept across launches for the app and profile and applied as the
  appearance says. A dark theme can only be the dark one and a light theme
  the light one; `"Default"` (the stock look with the stylesheet's colors)
  is either. A name that does not fit is the sender's error. View > Themes
  lists the themes for the appearance the window is in and picks for it.
  Views read `app.appearance.theme` (showing now), `light_theme`,
  `dark_theme`, and `themes`: every theme as `{ name, dark, background,
  foreground, accent, colors }` (hex strings; `colors` is red, green,
  yellow, blue, magenta, cyan), `"Default"` listed once for each, so a mod
  can draw its own picker:

```lua
-- cards for the dark themes, with color dots; a click picks one
local cards = {}
for _, theme in app.appearance.themes do
	if theme.dark then
		local dots = {}
		for _, color in theme.colors do
			table.insert(dots, ui.row { class = "dot", style = { background = color } })
		end
		table.insert(cards, ui.column { class = "theme-card",
			selected = theme.name == app.appearance.dark_theme,
			style = { background = theme.background },
			to = "app:theme", msg = { dark = theme.name },
			ui.text { theme.name, style = { color = theme.foreground } },
			ui.row { gap = "small", table.unpack(dots) },
		})
	end
end
```

- `app:sound`, `msg` `"default"` (a short chime the app makes) or a sound
  file (`.wav` `.aiff` `.aif` `.caf` `.mp3` `.m4a`, at most 2 MiB), named
  as `ui.image` names pictures: relative to the mod's folder, `~/...` or
  absolute, and inside the app's folders. Never a URL. Sounds closer than
  150 ms apart are dropped. macOS plays them (`NSSound`); Linux with
  `paplay` or `aplay` when installed; Windows plays none yet (logged).
- `app:notify`, `msg` `{ title, body?, id? }`: a system notification. macOS
  posts it through the Notification Center when the app runs as a bundle
  (an installed app, `just bundle`, `just bundle-app`), asking the user to
  allow notifications the first time; it shows even when the window is in
  front, so check `app.window.active` to alert only when the user is not
  looking. A run from source (`just run`, a bare binary), Linux and Windows
  show it as a toast in the window instead, for 5 s. Clicking either brings
  the app and its window forward and tells the mod that posted it, if it subscribes to
  `app/notification`: `{ topic = "app/notification", value = { id, clicked
  = true } }`. At most 5 notifications in 10 s are posted.

```lua
return mod {
	subscribe = { "app/notification" },
	update = function(state, msg, props)
		if type(msg) == "table" and msg.topic == "app/notification" then
			state.focused = msg.value.id      -- jump to the agent that finished
		elseif type(msg) == "table" and msg.finished then
			-- told by whatever watches the agents (a provider, another mod)
			return state, {
				{ to = "app:sound", msg = "default" },
				{ to = "app:notify", msg = { title = "Agent done", body = msg.finished.title, id = msg.finished.pane } },
			}
		end
	end,
	view = function(state) return ui.text "agents" end,
}
```

- `app:pick`, `msg` `{ kind = "open" | "folder" | "save", reply, multiple?,
  prompt?, types?, name?, folder? }`: the system's panel to open files
  (`types` keeps those extensions), pick folders, or name a file to save
  (`name` and `folder` suggest one). The answer comes back to the mod's
  `update` as `reply` with `paths` (a list; `path` for a save) or
  `cancelled = true` added. What the user picks, they chose to show that
  mod, so its `ui.image` (and its components') may show it from then on
  (folders: what is in them); other mods may not.

```lua
update = function(state, msg)
	if msg == "open" then
		return state, { { to = "app:pick", msg = { kind = "open", types = { "tif", "jpg" }, reply = { opened = true } } } }
	elseif type(msg) == "table" and msg.opened and msg.paths then
		state.file = msg.paths[1]
	end
end,
```

- `app:window`, `msg` a mod's name, `{ open = "<mod>", send = <msg>? }`
  or `{ close = "<mod>" }`: opens that mod in a window of its own (or
  raises it), as a control's `window = "<mod>"` does, or closes it. `send`
  reaches the mod's `update` (from the asking mod) before its window
  draws, so it opens on a page; a `send` to a mod that does not exist is
  the sender's error. The mod's `title` titles the window, and its
  `window = { size = { width, height }, min_size = { width, height } }`
  sizes it (both kept within the screen, as the main window's `size` is;
  420x560 and 280x240 when unset; a size takes effect the next time the
  window opens). Any mod may open or close any mod's window, as any
  control may: the window shows that mod's own view and `send` is an
  ordinary message, so nothing new crosses. The shell is the main window
  and is never opened this way. Close a mod's own window from its
  `update` with `{ close = "<mod>" }`: `app:command` `close-window` runs in
  the main window and would close that.

```lua
-- mods/settings/main.luau: Settings as a window, on the page asked for
return mod {
	title = "Settings",
	window = { size = { width = 860, height = 600 }, min_size = { width = 640, height = 480 } },
	init = function() return { page = "general" } end,
	update = function(state, msg)
		if type(msg) == "table" and msg.page then
			state.page = msg.page
		elseif msg == "done" then
			return state, { { to = "app:window", msg = { close = "settings" } } }
		end
	end,
	view = function(state) return ui.text(state.page) end,
}

-- elsewhere, a palette entry or another mod's update:
return state, { { to = "app:window", msg = { open = "settings", send = { page = "machines" } } } }
```

- A request the app cannot read (an unknown command, a misspelled
  `app:<what>`) is the sender's error, shown as an `update` error.
- Previews (the Components window) ask nothing, and a provider's answer
  cannot be passed off as a request.

## Pointer, dropped files and keys anywhere

A row or column hears more than clicks:

- `pointer = <msg>` is sent as a button goes down on it (`phase = "down"`),
  as the pointer moves while held, even outside it (`"move"`), and as it
  comes up (`"up"`), with `x`, `y` (fractions of its box as it was at the
  press), `dx`, `dy` (pixels since), `width`, `height`, `button`, `clicks`
  and the keys held added. The innermost one with `pointer` takes the
  press. `hover = true` also sends `"hover"` and `"leave"`.
- `range = { min, max, step?, value?, speed?, axis? }` adds the `value` a
  drag sets, so the view does no arithmetic: where the pointer is between
  `min` and `max` (a slider), or, with `speed`, `value` plus pixels times
  `speed`, counted from the value at the press (a scrubbed number).
- `wheel = <msg>` gets scrolls and trackpad pinches over it (`dx`, `dy`,
  `zoom`, `x`, `y`), and keeps them from scrolling what is around it.
- `drop = <msg>` gets files dropped from other apps (`paths`); the mod's
  `ui.image` may show them from then on.
- `cursor = "crosshair"` (`grab`, `grabbing`, `ew-resize`, `move`, ...).
- `area = { left, top, width, height }` places any row, column or panel at
  fractions of its parent: handles, markers, a crop frame. In a
  `ui.image`'s `overlay` those are fractions of the picture, wherever zoom
  and pan put it (`ui.image { zoom, center, overlay, measure }`).

Record undo on `phase == "up"`, not on every move. Before writing one,
look for a component: `slider`, `number-field`, `histogram` (draggable
markers), `curve-editor`, `image-viewer`.

A mod can take keys anywhere in the window, once nothing nearer (an open
overlay, a terminal) took them. Every mod, and every mounted copy of one
(a tab's), that takes a key hears it. While a text field has focus it
keeps all its keys (copy, select all, undo), so mods hear none:

```lua
return mod {
	keys = { ["secondary-1"] = "histogram", ["secondary-l"] = "log" },
	update = function(state, msg) ... end,
	view = function(state) ... end,
}
```

## Tabs

`tabbed { id = "main", start = "component:terminal" }` is a tab bar that
keeps its tabs: a tab is data (a kind and its props), never a file. The `+`
opens a menu of `app.kinds`; each tab shows its kind; tabs are saved and come
back at the next launch (a terminal starts a fresh shell in its folder, a
browser reopens its page). Every change is published on `tabs/<id>` as
`{ count, selected, title }`. The main area and the bottom pane each have
one (`mods/editor`, `mods/terminal`).

To offer something as a kind of tab, give its component or mod a `tab`:

```lua
tab = { title = "Terminal", icon = "terminal", props = { cwd = "~" } },
-- A browser also names the prop it reports through; the tab keeps the
-- `url` and `title` that come back:
tab = { title = "Browser", icon = "globe", props = { url = "https://luau.org" }, report = "navigate" },
```

A mod brings up a main tab of a kind by sending the shell
`{ to = "shell", msg = { open_tab = "mod:mockup" } }`: the open tab of that
kind is selected, else a new one opens (the shell sends `tabbed#main`
`{ open = kind }`). Send it from `init` when `props` is nil (the mod in its
own place, not its tabs) and only once, or every tab opens another.
`{ close_tab = "mod:mockup" }` closes every tab of the kind (`tabbed`'s
`{ close_kind = kind }`): send it before the mod's folder is deleted. A tab
whose mod is gone says so and waits to be closed.

A component kind is drawn with its tab's props and `id` (so a terminal or
browser is the tab's own); a mod kind is mounted as an instance per tab.
`offer = { "component:terminal", ... }` limits a bar's menu.

**The strip apart from its content.** `tabbed { id = "main", show = "bar" }`
draws only the strip, as the shell does in its title row. The bar publishes
`tabs/<id>` as it starts and at every change, with the selected tab's record
as `tab`; a mod subscribed to it draws that tab with `tab-view { tab, kind }`
(`mods/editor` does, finding `kind` in `app.kinds`). What the content reports
(`{ tab_report = id, url, title }`) goes back to the bar on
`topic:tabs/<id>/report`.

**Looks.** The strip's look comes from its `variant` prop, else the
stylesheet's `looks = { tabs = "..." }`, else `classic`. Built in: `pills`,
`underline`, `outline`, `cards`, `compact`, `classic`, `square`. Any other
name works too: it becomes the classes `.tabs-<name>` (the strip) and
`.tabs-<name>-tab` (each tab), so a stylesheet can define a look with rules
alone:

```lua
looks = { tabs = "soft" },
rules = {
	[".tabs-soft"] = { gap = 4, padding = 4 },
	[".tabs-soft-tab"] = { radius = 10, padding = { 6, 12 } },
	[".tabs-soft-tab:selected"] = { background = "accent/15", color = "accent" },
},
```

`curved` joins the selected tab to what is under the strip, as one piece: a
body in the `card` color, square at its foot, with shoulders flaring out into
the card (`.tabs-shoulder-left` and `-right`, colored as the strip's
background). It only reads right with a card of the same color directly
under the strip, so with `looks = { tabs = "curved" }` the shell moves its
title bar over the center column (each side panel gets a bar of its own),
drops the bar's bottom line (`.toolbar-joined`), and the editor's card
starts right under it with no border (`.tab-card-joined`). A strip of your
own in that look needs the same: no gap, no border between it and a `card`.
Tabs open and close with the `.tabs-tab` rule's transition (see
"Arriving and leaving"); set it to `"none"` to stop that.

## Loop

1. Edit or create `.luau` files under `app/`.
2. Run `just check`. It loads every module headlessly, the user's folder
   included (`just check-builtin` skips it), and prints
   `ok: ...` or one `module: problem` line per module. Fix the problems until
   it prints `ok`. The messages give the exact place, e.g.
   `explorer: view: panel[2].item: unknown key `lable`; it takes ...`.
3. If the window is running, it has already reloaded. Module state is kept
   across reloads. To see the result, run `just screenshot`, then Read
   `target/screenshot.png`.
4. The user often has their own window open (`just run` or `cargo run`).
   Never stop it: no `pkill`/`killall` by name, which also kills theirs.
   If you need a window of your own, start the built binary in the
   background, note its PID (`target/release/moonframe app & echo $!`),
   capture it with `just screenshot target/screenshot.png <PID>`, and stop
   only that PID with `kill <PID>`. Their window reloads from the same files
   anyway, so often you don't need your own.
5. Never type into, click or bring forward any window to test (no
   `osascript … keystroke`, no focus changes): input goes to whatever app
   the user has in front, and they are usually working. Test behaviour
   with `just check`, unit tests and screenshots by PID; ask the user to try
   interactions themselves.
6. Every change the window reloads for is kept as a snapshot of the user's
   folder (or the profile's), whoever made it, so you need not commit as
   you go. When the user asks to take a change back, use
   `moonframe [--profile NAME] history undo` (`redo` to go forward again,
   `list` and `restore ID` for an older one, `show ID` for its patch); the
   window reloads with the files it writes back. Undo leaves
   `settings.toml`, databases and installed packages as they are. The user
   can do the same from View > Undo Last Change (⌥⌘Z) in Developer Mode.

`just types` type-checks every `.luau` file in `app/` against
`types/moonframe.d.luau` in strict mode (set in `app/.luaurc`). It runs in
`just ci`, and downloads the pinned luau-lsp into `target/tools` the first
time. Run it after `just check`.
- It catches unknown `ui` functions, wrong token values (`tone = "loud"`),
  and wrong value types before anything runs.
- It does not catch misspelled keys (`lable`), because Luau allows extra
  fields in a table literal. `just check` catches those.
- Annotations are optional. Add them where they help, e.g.
  `type State = { open: boolean }` and `init = function(previous: State?): State`.
- When you add an element or token to the host, add it to
  `types/moonframe.d.luau` too. A Rust test fails if they drift apart.

Mistakes never move the layout:

- **A bad element** (unknown key, value or icon) is drawn as a small red
  warning mark in its place, with the message on hover. The rest of the view
  draws as before. Its error is still a problem in `just check` and the log,
  so fix it.
- **A broken module** (syntax error, failing `view`) keeps running its last
  good version, with its error floating over the corner of that view instead
  of pushing it down. Stylesheet and provider errors float over the top of
  the window the same way.
- **A broken copy in `~/.config/moonframe`** of a built-in mod or shell
  that has never loaded falls back to the app's own version, with the error
  saying so, rather than leaving a hole.
- Only the view's root, or going past the size and depth limits, fails a
  view as a whole.

An unknown icon's error names the closest bundled icons. Check
`IconName` in `types/moonframe.d.luau` before picking one.

## A module

Each file returns `mod { ... }`:

```lua
return mod {
	region = "left",   -- optional: the shell region to draw this mod in
	styles = {         -- optional: style rules for what this mod draws (see Stylesheet)
		[".row"] = { padding = { 4, 8 } },
	},
	order = 0,         -- optional: position in the region, lower first, then by name
	title = "Files",   -- optional: informational

	-- Optional. `previous` is the state from before a reload, or saved from
	-- the last run (nil the very first time). `props` is what an instance was
	-- mounted with (nil for a mod in its own place); see "Instances".
	init = function(previous, props)
		return previous or { selected = nil }
	end,

	-- `init` may also return sends, as `update` does, delivered once loaded.
	-- Optional. Return the new state, or nothing to keep the table you changed.
	-- A second result sends messages on: { { to = "mod", msg = ... }, ... }
	update = function(state, msg)
		if type(msg) == "table" and msg.open then
			state.selected = msg.open
			return state, { { to = "preview", msg = { show = msg.open } } }
		end
	end,

	-- Required. Returns one element. `app` is
	-- { dark = bool, appearance = { mode = "system"|"light"|"dark", dark, theme,
	--   light_theme, dark_theme, themes = { { name, dark, background, ... } } },
	--   mods = { names }, regions = { name = { mod names } },
	--   kinds = what tab bars can open (see "Tabs"), looks = the stylesheet's looks,
	--   window = { platform = "macos"|"linux"|"windows", titlebar = "system"|"hidden",
	--              maximized, fullscreen, active },
	--   <provider> = what each provider last published, e.g.
	--   system = { cpu, cpu_history, memory, memory_used, memory_total,
	--              received, transmitted }, git = { repository, branch, changed, ahead, behind } }
	-- Provider data is nil until its first publish.
	view = function(state, app)
		return ui.panel { title = "Files", grow = true, ui.text "hello" }
	end,
}
```

Rules that trip agents up:

- Unknown element types, keys and token values are errors, not ignored.
  `just check` names them.
- The keys allowed on the `mod { }` table are only `init`, `update`, `view`,
  `region`, `order`, `title`, `styles`, `subscribe` (see "Topics"), `tab`
  (see "Tabs") and `window`, plus `menus` and `about` in `main.luau` only
  (see "Menus and About"). In `main.luau`, `window` is the main window's
  frame (see "Window and title bar"); in another mod it is the size of the
  mod's own window, `window = { size = { width, height }, min_size = {
  width, height } }` (see "Asking the app", `app:window`). `update` and
  `view` also get `props` last.
- State and messages must be plain data: nil, booleans, numbers, strings,
  lists, and string-keyed tables. Functions in state stop it surviving a
  reload. A table is a list without gaps or has string keys, never both.
- There is no `require`, `print`, `os`, `io` or `debug`, and each mod is one
  file; share code through components and `use`. Use `log(...)` to print to
  the terminal that ran `just run`.
- Children are the array part of the table. `nil` and `false` children are
  skipped, so `cond and ui.text "x"` works. For a computed list, build a
  table and pass `table.unpack(items)` as the last entry.
- Each call has an instruction budget, so an endless loop is stopped with an
  error instead of freezing the window.
- The mod name is the folder name: `a-z`, `0-9`, `-` and `_`. `shell` is
  reserved for `main.luau`.

## Elements

Text-like elements take their main value positionally:
`ui.text "hi"`, `ui.text { "hi", tone = "muted" }`, `ui.button { "Save", msg = "save" }`.

| Element | Keys (besides children) | Notes |
| --- | --- | --- |
| `ui.row { ... }`, `ui.column { ... }` | frame keys, `align`, `justify`, `wrap`, `drag`, `selected`, `key`, extras, and `msg`/`to`, `command` or `window` (a mod to open in a window of its own, as the title bar's layout button opens `mods/looks`; give the mod no `region` and a `title`, which titles the window, and a `window = { size, min_size }` to size it; `window` takes no `msg`, so to open it on a page send `app:window` instead) | flex containers; with `msg` or `command` the whole stack is clickable (style it with `.class:hover`). `drag = true` makes it a title bar: dragging it moves the window, a double click zooms it, and buttons and clickable stacks inside still take their clicks (see "Window and title bar"). Defaults: `gap = "normal"`, `padding = "none"` |
| `ui.panel { ... }` | frame keys, `title`, `actions = { elements }` | header (title plus right-aligned actions) over a column body. Defaults: `gap` and `padding` `"normal"` |
| `ui.hsplit { ... }`, `ui.vsplit { ... }` | `id`, `divider` | user-resizable; children must be `ui.pane`. Always grows to fill. `divider = false` drops the hairline between panes (cards on a background), showing a handle only on hover |
| `ui.pane { ... }` | `size`, `min`, `max`, `fixed`, `visible`, `lazy`, `gap`, `padding`, `surface`, `scroll` | only inside a split; content is a column. `fixed = true` keeps its size on window resize and needs `size`. `lazy = true` starts what it mounts only once it first shows and keeps it asleep (neither run nor drawn, state kept) while hidden: a heavy side panel or tab |
| `ui.list { count = #rows, row_height = 24, row = function(i) ... end }` | `id`, `count`, `row_height`, `row`, frame keys, `class`, `style` | rows of one height of which only those that show are built and drawn: use it for anything that can grow long (files, logs, search results, history). `row(i)` builds row `i` (from 1) and is called again as the list scrolls, so build the data in the view and keep `row` cheap. It scrolls itself and takes the space it is given (or its `height`). Two lists in one mod need two `id`s. See "Long lists" |
| `ui.text "..."` | `text`, `tone`, `size`, `weight`, `truncate`, `lines`, `wrap`, `strike`, extras | a bare string child is plain text |
| `ui.button { "Label", ... }` | `label`, `icon` (alone: icon-only), `variant`, `small`, `disabled`, extras, and `msg`/`to` or `command` | |
| `ui.item { "Label", ... }` | `label`, `detail`, `badge`, `selected`, extras, and `msg`/`to` or `command` | clickable list row for sidebars |
| `ui.badge "..."` | `text`, `tone`, extras | pill |
| `ui.terminal { id = "main", cwd = "~/code", engine = "alacritty" }` | `id`, `cwd`, `engine` (`alacritty` or `ghostty`), `backend` (`auto`, `herdr-core`, `tmux`, `pty`), `class`, `style` | a live shell (the user's login shell) that fills the space it gets; see "Terminals" |
| `ui.view { name = "editor", id = "main", props = {...} }` | `name`, `id`, `props`, `class`, `style` | a view the app writes in Rust (apps on the library only; the starter registers none), kept by mod and `id`, told when `props` change; its messages reach this mod's `update`. See "Extending the host" |
| `ui.browser { id = "docs", url = "https://example.com" }` | `id`, `url`, `zoom`, `navigate`, `loading`, `class`, `style` | a web page in a native view that fills the space it gets; see "Browsers" |
| `ui.markdown "# Notes"` | `text`, `link`, `to`, `grow`, `scroll`, `width`, `height`, `max_width`, `class`, `style` | markdown the host renders (headings, emphasis, lists, task lists, tables, quotes, highlighted code), up to 1 MiB; a link click sends `link` with `url` added, nothing opens by itself; pictures are not loaded |
| `ui.input { id = "q", value = state.q, change = "q" }` | `id`, `value`, `placeholder`, `change`, `submit`, `keys`, `to`, `multiline`, `rows`, `disabled`, `focus`, `mono`, `width`, `grow`, `browser`, `class`, `style` | a text field the host edits (caret, selection, IME, undo); see "Text input" |
| `ui.icon "cpu"` | `name`, `tone`, `size`, `spin`, extras | a bundled icon at the text's size and color: Lucide glyphs (`git-branch`, `git-pull-request`, `circle-dot`, `terminal`, `pin`, `trash-2`, ...), agent logos `agent-claude` `agent-codex` `agent-gemini` `agent-cursor` `agent-copilot` `agent-opencode` `agent-amp` `agent-droid` `agent-kiro` `agent-qwen` `agent-grok`, and brands `brand-anthropic` `brand-openai` `brand-mistral` `brand-ollama` `brand-github` `brand-gitlab`. An unknown name is an error that lists them all (also in `types/moonframe.d.luau` as `IconName`). `spin = true` turns it, for loaders |
| `ui.image "avatar.png"` | `src`, `width`, `height`, `fit` (`contain` `cover` `fill` `scale-down` `none`), extras, `class`, `style` | a picture from a file: `src` is relative to the drawing mod's folder, `~/...`, or absolute, but must be inside the app's folders (`app/`, `~/.config/moonframe`) or the image cache `$XDG_CACHE_HOME/Moonframe/images` (default `~/.cache/moonframe/images`), at most 16 MB. Not a URL (the host has no HTTP client and views must not fetch): have a provider download into the image cache and pass the path. A missing or disallowed file draws an empty box of `width` x `height`, not an error. A `radius` rule rounds it (avatars); `fit = "cover"` fills a square |
| `ui.line { points = { 3, 5, 2, 8 } }` | `points`, `min`, `max`, `baseline`, `area`, `dots` (`last` `all`), `stroke`, `dashed`, `width`, `height`, `class`, `style` | a line through values evenly spaced across its box (fills its parent without `width`/`height`): trends, curves in tables, forecasts. Style `color` strokes it, `background` fills the area to `baseline` (with `area = true`), `border_color` draws the baseline dashed. Use it for any line or curve, a dashed rule (two equal points with `dashed = true`) included |
| `ui.hatch { width = 40, height = 12 }` | `gap`, `width`, `height`, `grow`, `class`, `style` | diagonal stripes in style `color` over `background`: what is held back, pending or still missing (in a stacked bar, under a floor) |
| `ui.divider()` | | a line across the parent's axis |
| `ui.spacer()` | | takes the free space, e.g. to push items right |
| `ui.region "name"` or `ui.region { name = "x", direction = "row", gap = "large" }` | `name`, `direction`, `gap`, `justify` | draws every mod with `region = name`, in order; `justify = "end"` pushes them to the far edge |
| `ui.mount "name"` | `mod` | draws one mod's view by name |
| `ui.popover { ..., content = el }` | `open`, `dismiss`, `to`, `side` (`top`/`bottom`/`left`/`right`), `align` (`start`/`end`: the anchor's left or right edge above and below it, its top or bottom beside it), `gap`, `width`, `keys`, `content` | floats `content` over everything beside its anchor (its children). While `open`, a click anywhere outside, or Escape, sends `dismiss` to this mod. Styled and animated by the `popover` rule |
| `ui.overlay { ..., content = el }` | `open`, `dismiss`, `place`, `backdrop`, `width`, `height`, `keys`, `content`, `class`, `style` | floats `content` over the whole window, above popovers: a dialog, palette, sheet or toast (see "Dialogs, palettes, sheets and toasts"). No children |

**Frame keys** (`row`, `column`, `panel`):

- `grow = true` takes the free space along the parent's axis. Use it on the
  root of every view that should fill its pane.
- `width` and `height` are pixels.
- `gap` and `padding` take a space token.
- `surface` sets the background and `border` the edges.
- `scroll = true` scrolls along the container's axis; `"x"`, `"y"` or
  `"both"` pick the axes (long code lines that scroll sideways). Give the container
  `grow = true` or a fixed size so it has something to overflow.
- `shrink = false` keeps the content's size when the parent is short of
  space (strips, toolbars), instead of being squeezed.
- `self_align = "start"` (or `center`, `end`, `stretch`) places it across
  the parent's axis over the parent's `align`: a column stretches its
  children by default, so a capsule or track that should keep its own
  width says `self_align = "start"`.
- `hug = true` is usually better for small things (pills, badges, dots):
  its own width in a column, while in a row it follows the row's `align`
  (`self_align = "start"` in a row lifts it to the top).
- `max_width` and `max_height` cap it; `grow` with a `width` or `height`
  grows from that size.
- `float` takes it out of the flow and places it over its parent:
  `float = "top-right"` pins it to an edge or corner (`top-left`,
  `top-right`, `bottom-left`, `bottom-right`, `top`, `bottom`, `left`,
  `right` (an edge stretches along it), `fill`); `float = { at =
  "bottom-right", inset = 8 }` sets it in from there; `float = { top = -4,
  right = -4 }` gives pixel offsets (negative overhangs): count bubbles on
  an icon's corner, badges on avatars.
- A click goes to the innermost clickable element only: a close button in
  a clickable tab does not also select the tab.

**Tokens:**

- space: `none` `small` (4px) `normal` (8px) `large` (16px)
- tone: `normal` `muted` `accent` `success` `warning` `danger`
- size (text): `small` `normal` `large` `title` (1.5x) `headline` (1.75x) `display` (2.25x: a page's greeting) `hero` (3x: hero figures on dashboards)
- weight: `light` (300) `normal` (400) `medium` (500) `semibold` (600) `bold` (700)
- align: `stretch` `start` `center` `end`
- justify: `start` `center` `end` `between`
- surface: `background` `sidebar` `muted` `secondary`
- border: `true`, `all`, `top`, `bottom`, `left`, `right`, `none`
- direction: `row` `column`
- variant (button): `normal` `primary` `ghost` `outline` `danger`
- command: `quit` `reload` `toggle-theme` (light to dark or back, keeping the themes) `show-themes` `minimize` `maximize` (zoom or maximize, or restore) `close-window` `check-for-updates` (the app menu's Check for Updates) `inspect` (Inspect Element for the mod: see "Here")
- place (overlay): `center` `top` `bottom` `left` `right` `top-start`
  `top-end` `bottom-start` `bottom-end`
- browser commands (with `browser = "<id>"`): `browser-back`
  `browser-forward` `browser-reload` `browser-stop` `browser-open`

**Extras** (`row`, `column`, `button`, `item`, `text`, `icon`, `image`,
`badge`): `tooltip = "text"` shows
under the pointer (styled by the `tooltip` rule, whose `transition` eases
it in and whose delay, `"slide-fade fast ease-out 400ms"`, is how long the
pointer rests before it shows); `context = <data>` is sent
on a right-click, to open a context menu (with `to` like `msg`);
`copy = "text"` puts the text on the clipboard when clicked. `selected =
true` on a row or column matches `:selected` rules, for tabs and segments.

**Actions:** `msg = <data>` calls `update(state, msg)` on the mod that drew the
control. Add `to = "name"` to send it to another mod, or to `"shell"`.
`command = "..."` runs an app command instead. Use one or the other, not both.
To run a command from `update`, without a click, send it to the app:
`{ to = "app:command", msg = "minimize" }` (see "Asking the app").
A browser command also takes `browser = "<id>"` (and `to` for another mod's
browser).

## Stylesheet

`app/style.luau` returns `style { ... }`. A broken stylesheet keeps the
previous one and shows its error at the top of the window, and in
`just check` under `style`.

```lua
return style {
	font_size = 14,      -- base text size; text sizes scale from it
	font_family = "Inter",
	-- The fixed-width font: terminals, and `font = "mono"` text (family and size).
	-- Fallbacks are searched for characters the family lacks, such as Nerd Font
	-- icons in prompts and status lines; the default is { "Symbols Nerd Font Mono" }.
	mono = {
		family = "JetBrainsMono Nerd Font Mono",
		size = 13, weight = 400, line_height = 1.3,
		fallbacks = { "Symbols Nerd Font Mono" },
		ligatures = false,
	},
	radius = 8,          -- the theme's corner radius, which buttons use
	speeds = { drawer = "220ms" }, -- named durations (fast/normal/slow exist already)
	animation_speed = 1, -- every animation: 2 twice as fast, 0.5 half, 0 off

	colors = {
		-- A theme color's name replaces it app-wide (buttons and other controls too)
		-- in the Default theme; a theme picked in View > Themes brings its own:
		accent = { light = "#4f46e5", dark = "#818cf8" },
		card = { light = "#ffffff", dark = "#1e1e22" },
		-- Any other name is a new color for use in rules and styles:
		brand = "#ff3366",   -- one value for both modes
	},

	rules = {
		panel = { background = "card", radius = 12, margin = "small", border = true },
		["panel-header"] = { border = "none", color = "accent" },
		["item:selected"] = { background = "accent/15", color = "accent" },
		["item:hover"] = { background = "accent/8" },
		[".hero"] = { size = "title", weight = "bold", color = "accent" },
	},
}
```

**Fonts.** `font_family` and `font_size` are the interface font; `mono` is
the fixed-width one. Terminals use every `mono` key. Text with
`font = "mono"` (and `mono = true` inputs) uses only its `family` and `size`.

| `mono` key | Default | Takes |
|---|---|---|
| `family` | `"Menlo"` | a family name, as `fc-list` prints it |
| `size` | `13` | 6 to 64 |
| `weight` | regular | 100 (thin) to 900 (black), 400 regular, 700 bold; bold text in the terminal still draws bold |
| `line_height` | `1.3` | 0.8 to 3, a multiple of `size` |
| `fallbacks` | `{ "Symbols Nerd Font Mono" }` | fonts searched in order for characters the family lacks; `{}` for none |
| `ligatures` | the font's own | `false` keeps `->`, `!=` and the like apart |

Picking a font for the user:
- **Use a name that is installed.** A misspelled or missing family is not
  an error, and the text just draws in another font. List the installed ones with
  `fc-list : family | sort -u` (on macOS without fontconfig:
  `system_profiler SPFontsDataType | grep 'Family:' | sort -u`), and
  copy the name exactly, spaces included.
- **Nerd Font icons** (vim and neovim status lines, starship, lsd, tmux
  themes) need a font that has them. Either keep any family and rely on
  the `Symbols Nerd Font Mono` fallback (`brew install --cask
  font-symbols-only-nerd-font`), or set `family` to a patched font.
- **Pick the "Mono" variant of a patched font** for terminals:
  `"JetBrainsMono Nerd Font Mono"`, not `"JetBrainsMono Nerd Font"` or
  `"... Propo"`. Those draw icons wider than one cell, and the columns
  after them drift.
- Set `fallbacks` only to add fonts (say, a CJK one) or to drop the
  default. The list replaces the default, so keep
  `"Symbols Nerd Font Mono"` in it when icons matter.
- For the installed app, edit `~/.config/moonframe/style.luau`, not the
  repo's `app/style.luau`. Changes apply live, and open terminals re-fit their grid
  to the new cell size.

**Selectors.**
- An element type: `row` `column` `panel` `panel-header` `pane` `text`
  `button` `item` `badge` `icon` `image` `line` `hatch` `terminal` `browser` `input` `markdown` `markdown-code` `markdown-quote` `divider` `popover` `tooltip`
  `overlay` `overlay-backdrop`. `panel-header` is a panel's title bar;
  `tooltip` is every tooltip's box; `overlay-backdrop` is what dims the
  window behind an overlay with `backdrop = true` (its `background` and
  `opacity`; default black at 35%).
- Parts of an input the editor paints: `input-placeholder` (its `color`),
  `input-selection` (its `background`, selected text) and `input-caret`
  (its `background`, the text cursor). They default to the muted
  foreground and the accent.
- A class: `.name`.
- Either one can take a state:
  - `:hover`: under the pointer. Hover rules only change `background`,
    `color` and `border_color`.
  - `:selected`: applies to `ui.item` and to a row or column with
    `selected = true`.
  - `:focus`: has keyboard focus; applies to `ui.input`. A focused input
    ignores its hover rules.
  - `:disabled`: `disabled = true`; applies to `ui.input` and
    `ui.button`. A disabled element has no hover.
  - `:<name>`, any other plain name: a state of the view's own, which an
    element enters with `state = "<name>"` (see "States").

  A state's rule layers over the same selector's resting rule, so
  `["input:hover"] = { border_color = "accent" }` keeps the `input` rule's
  background.

**Classes and inline styles.** These elements take `class = "a b"` (or
`{ "a", "b" }`), `style = { ...same properties... }` and `state = "name"`:
`row`, `column`, `panel`, `pane`, `text`, `button`, `item`, `badge`, `icon`,
`image`, `terminal`, `browser`, `input`, `list`, `popover`, `overlay` (on its
content).

**States.** As in QML, a view says which state an element is in, and the
stylesheet says what each state looks like and how to move between them:

```lua
-- view
ui.column { class = "drawer", state = state.open and "open" or "closed", ... }
-- style.luau (or the mod's `styles`)
[".drawer"] = { transition = "normal ease-out", background = "sidebar" },
[".drawer:open"] = { width = 280 },
[".drawer:closed"] = { width = 48, opacity = 0.8 },
```

A state's rule layers over the resting rule (after `:selected`). With a
`transition`, a change of state eases `background`, `color`,
`border_color`, `opacity`, `width` and `height` from the old state's values
to the new; turning back mid-move starts from where it is. A property only
one of the two states sets jumps. Prefer a state over picking classes in
the view when the look changes with the element's mode (open/closed,
compact/full, idle/busy): the view stays about meaning, the stylesheet
about looks, and the user can restyle each state.

**Numbers that change.** Text is proportional, so a reading such as a rate
or a percentage changes width as it ticks and pushes its neighbours. Put it
in a fixed-width box: `ui.row { width = 64, justify = "end", ui.text { rate } }`
(see `app/mods/system`).

**Mod styles.** A mod can bring its own rules under `styles = { selector = { ... } }`.
They use the same selectors, properties and colors (including palette names
from `style.luau`), but **apply only to elements that mod draws**, so
an extension can't restyle the rest of the app. An extension should style
itself this way, ideally with classes named after it, rather than asking
for edits to `style.luau`.

**Precedence**, lowest first:
1. the element's built-in default
2. type rules: the mod's, then `style.luau`'s
3. class rules, in the order the element lists them, each the mod's then
   `style.luau`'s
4. its `style = { }`
5. its own keys (`padding = ...`, `surface = ...`, `tone = ...`, `border = ...`, `size = ...`)

So `style.luau` always outranks a mod at the same step (the app can restyle
any extension), and a mod's class rule outranks `style.luau`'s type rule.
Leave an element's own keys off if the stylesheet should decide.

**Properties.**

| Property | Value |
| --- | --- |
| `background`, `color` (text, inherited), `border_color` | a color |
| `border` | `true`, `all`, `top`, `bottom`, `left`, `right`, `none` |
| `radius` | pixels, or each corner clockwise from the top left as in CSS: `{ 8, 8, 0, 0 }` rounds the top only, `{ a, b }` gives the top left and bottom right `a` |
| `padding`, `margin` | a space token or pixels for every edge, `{ y, x }`, `{ top, right, bottom, left }`, or named edges `{ x = 12, top = 4 }` (edge names win over `x`/`y`) |
| `gap` | a space token or pixels |
| `shadow` | a preset (`small` `medium` `large`, or `none` to remove one set by a lower rule), a CSS box-shadow string, a layer table, or a list of layers (see below) |
| `size`, `weight` | text tokens (inherited) |
| `font` | `sans` or `mono` (the terminal font, for code and paths; inherited) |
| `case` | `upper`, `lower` or `none`: the letter case a `ui.text` shows (headings in capitals, restyleable) |
| `gradient` | a linear gradient over the background: `"180deg, #ffa53a, #ff6200"` (CSS angles: 0 runs up, 180 down, the default; the angle is optional), or `{ from, to, angle }`. On any element `background` takes; `:hover` and `:selected` rules can set one too |
| `opacity` | 0 to 1 |
| `width`, `height` | pixels, under the element's own `width`/`height`; on rows, columns, panels and lists. What a named state changes: `[".drawer:closed"] = { width = 48 }` |
| `transition` | how a `pane`, `popover` or `overlay` animates when it shows or hides, how a keyed row or column arrives and goes, and how an element with a `state` eases between states (see below) |
| `delay` | milliseconds or `"40ms"`: replaces the transition's delay for a change of `state`. Set it inline from an item's index so a list's items move one after another (see "Staggered states") |
| `clip` | `true` hides what overflows the box instead of drawing it past its edges: a column whose width eases down (a sidebar folding to a rail) |

**Shadows.** Write them like CSS `box-shadow`. Lengths are pixels, with or
without `px`, and layers are separated by commas. The color can be any
color below; the default is black at 25%. Hex and names only, not `rgba()`.

```lua
panel = { shadow = "0 4px 12px -2px #00000033" },              -- x y blur spread color
[".lifted"] = { shadow = "0 1 2 #0002, 0 8px 24px accent/20" }, -- two layers
[".pressed"] = { shadow = "inset 0 2 4 #0003" },                -- inside the edge
pane = { shadow = { y = 2, blur = 6, color = "accent/30" } },   -- one layer as a table
row = { shadow = { "0 1 2 #0002", { y = 8, blur = 24 } } },     -- a list of layers
[".flat"] = { shadow = "none" },                                -- remove it
```

A table layer takes `x`, `y`, `blur`, `spread`, `color` and `inset`; missing
lengths are 0. A string layer takes 2 to 4 lengths (`x y [blur [spread]]`),
then the color, with an optional `inset` first.

**Transitions.** When a pane's `visible` flips (for example from a "Hide
left" button), it animates by its `transition` instead of jumping:

```lua
pane = { transition = "slide 220ms ease-out" },                   -- every pane
[".drawer"] = { transition = "slide-fade 0.3s cubic-bezier(0.2, 0, 0, 1)" },
[".popover"] = { transition = { kind = "fade", duration = 150, curve = "linear" } },
[".instant"] = { transition = "none" },
```

- **Kinds:** `slide` (the size grows from or shrinks to nothing), `fade`
  (opacity, then it takes or gives back its space), `slide-fade` (both),
  `none`.
- **Curves:** `linear` `ease` `ease-in` `ease-out` `ease-in-out`
  `cubic-bezier(x1, y1, x2, y2)`, as in CSS.
- **Duration:** `220ms`, `0.22s`, or a **speed name** in the string;
  milliseconds or any of those strings in the table; up to 10s.
- **Speeds:** `fast` (120ms), `normal` (220ms) and `slow` (400ms) always
  exist. `speeds = { name = "90ms" | 90 }` at the top of `style.luau`
  redefines them or adds names; a name can't be a kind or a curve. Prefer
  speed names over raw durations so one edit retimes the whole app.
- **`animation_speed`:** at the top of `style.luau`, it divides every
  duration (mod `styles` and inline `style` included): `2` is twice as fast,
  `0` turns animations off.
- **Delay:** a second duration, as in CSS, waits before a change of `state`
  starts moving: `"fade 150ms ease-out 40ms"`, or `delay = 40` in the
  table. (Panes, popovers and keyed children ignore it.)
- **Defaults:** slide, the `normal` speed, `ease`, no delay.

Words can come in any order. Toggling again mid-animation turns back from
where the pane is. A slide moves across the pane's `size` (or `max`), not a
width the user dragged it to. Put the transition on the pane (type rule,
class, or `style`), not on what is inside it.

**Staggered states.** Items that change state together (labels fading as a
sidebar folds) can move one after another: keep the transition in the
rules, and give each item its place in line as an inline `delay`, which
replaces the transition's. Each state's rule may bring its own transition,
so going one way can be quicker than coming back:

```lua
for index, entry in entries do
	ui.text { entry.label, class = "label", state = if folded then "hidden" else "shown",
		style = { delay = (index - 1) * 16 } }
end
-- styles: [".label"] = { transition = "fade 180ms ease-out" },
--   [".label:shown"] = { opacity = 1 },
--   [".label:hidden"] = { opacity = 0, transition = "fade 90ms ease-in" },
```

The `side-nav` component does this for a whole sidebar, with the column's
width easing between `.side-nav:full` and `.side-nav:rail` (and `clip` on
it, so the labels do not spill past the narrowing edge).

**Arriving and leaving.** A `ui.row` or `ui.column` with a `key` (1 to 64 of
a-z, 0-9, `-` and `_`, unique among its siblings) stays itself as its
siblings come and go. Give it a `transition` rule and it plays in when its key
appears, and when the key goes its last drawing stays where it was, playing
out without taking clicks. `slide` grows or shrinks it along its parent's
axis (from its measured size), `fade` fades it, `slide-fade` does both. Its
container must already be on screen: what a container holds as it first
appears simply is there. Tabs do this (`.tabs-tab` has
`transition = "slide-fade fast ease-out"`); key the items of any list that
gains and loses entries the same way:

```lua
for _, note in state.notes do
	ui.row { key = note.id, class = "note", ui.text { note.title } }
end
-- style: [".note"] = { transition = "slide-fade fast ease-out" }
```

**Colors.**
- `#rgb`, `#rrggbb` or `#rrggbbaa`.
- A name from `colors`.
- A theme color: `background` `foreground` `muted` `muted-foreground`
  `sidebar` `card` `secondary` `border` `accent` `accent-foreground` `success`
  `warning` `danger` `list-hover` `list-active`. These follow light and dark
  mode, and the theme picked in View > Themes.
- Any of the above with `/NN` for percent opacity, e.g. `accent/15`.

Padding and margin layer **per edge**: a class with `padding = { top = 20 }`
changes only the top and keeps the other edges from the type rule. An
element's own `padding = "small"` key sets every edge.

**Where each element takes the properties.**
- On a `panel`, `padding` and `gap` apply to the body under the header.
- `button` takes `background`, `color` and `radius`. Setting a background or
  color overrides its `variant`.
- `divider` takes only `background`.

## Window and title bar

The shell (`main.luau` only) can ask for a window without the system title
bar and draw its own:

```lua
return mod {
	window = { titlebar = "hidden", traffic_lights = { x = 12, y = 12 } },
	view = function(state, app)
		local w = app.window
		local mac = w.platform == "macos"
		return ui.column { grow = true, gap = "none",
			ui.row { class = "titlebar", drag = true, height = 38, align = "center",
				-- room for macOS's traffic lights (about x + 70), unless fullscreen hides them
				mac and not w.fullscreen and ui.row { width = 80 },
				ui.text { "My app", weight = "medium" },
				ui.spacer(),
				-- Linux and Windows draw no window buttons when the title bar is hidden.
				w.titlebar == "hidden" and not mac and ui.row { gap = "none",
					ui.button { "–", variant = "ghost", small = true, command = "minimize" },
					ui.button { w.maximized and "❐" or "□", variant = "ghost", small = true, command = "maximize" },
					ui.button { "×", variant = "ghost", small = true, command = "close-window" },
				},
			},
			-- the rest of the layout
		}
	end,
}
```

- `size = { width = 1500, height = 960 }`: the size the window opens at
  (kept within the screen), for apps whose layout needs room.
- `titlebar = "hidden"`: on macOS the layout runs under a transparent title
  bar and the traffic lights stay, at `traffic_lights` (the first button's
  offset from the top left; the system's place when unset). On Windows the
  system frame goes. On Linux the window asks for client-side decorations;
  the app then draws a thin border with resize edges and a shadow, and a
  compositor that insists on its own frame keeps it (then
  `app.window.titlebar` says `"system"`). `"system"` (the default) is the
  normal title bar.
- **It applies when the window opens.** GPUI can't change a window's frame
  later, so after editing `window`, restart the app (the window says so).
- Give the layout a `drag = true` row: with the title bar hidden it is the
  only way to move the window. Double-clicking it zooms (macOS follows the
  user's "double-click a title bar" setting); right-click on Linux opens the
  window menu.
- `app.window` updates as the window is maximized, goes fullscreen, or
  gains and loses focus: dim the title bar on `not app.window.active`.
- `app/main.luau` ships without the system title bar: its top row is the
  title bar, with the main tab strip in it. Reserve room for the traffic
  lights only when `app.window.titlebar == "hidden"` (until the restart that
  applies `window`, the system bar is still there and the room would be a
  gap).

## Menus and About

The shell (`main.luau` only) can add menus to the menu bar and say what the
About panel shows. The host puts its app menu first (About <App>, Quit, named
after the app's title), then the shell's menus, then View (a shell menu titled
"View" merges into it), Components outside a release, and the shell's Window
and Help last.

```lua
-- A typed local: items with `msg` and with `command` mix in one list.
local MENUS: { { title: string, items: { any } } } = {
	{ title = "File", items = {
		{ label = "New Shell", msg = "new", to = "agent", shortcut = "secondary-t" },
		"-",
		{ label = "Close File", msg = "close-current", to = "editor", shortcut = "secondary-w" },
	} },
	{ title = "Window", items = { { label = "Minimize", command = "minimize", shortcut = "secondary-m" } } },
}

return mod {
	about = { version = "1.2", description = "What it is for.", website = "https://…", copyright = "…" },
	menus = MENUS,
	-- ...
}
```

- An item sends `msg` to the shell, or to `to`, as a click from the shell
  would, or runs an app `command`. `"-"` is a separator.
- `shortcut` is a key named as `keys` names them (`secondary-t` is ⌘T on
  macOS, Ctrl-T elsewhere); it shows in the menu and works while the window
  has focus. A focused terminal keeps plain and Ctrl keys, not ⌘ ones.
- A menu `shortcut` may also be one key that types text (`j`, `/`,
  `shift-u`), as a mail client's keys are: it works only while no text
  field or terminal has focus, so typing is never taken, and the menu does
  not show it (name it in the label or a hint). `examples/tidings` uses J
  and K this way. Overlay and field `keys` still refuse such keys.
- About opens macOS's own About panel (the app's icon and name, `version`,
  `description` and `website` as credits, `copyright`), and a toast
  elsewhere.
- The app's name in the menu bar is its title (`title` in moonframe.toml);
  menus and keys change when the shell reloads.

## Patterns

**Long lists.** Anything that can grow past a screenful (files, logs,
results, history) goes in a `ui.list`. Build the rows as plain data in the
view, then draw one in `row`; only the rows showing (and a few around them)
are ever built, and the host asks for more as the user scrolls:

```lua
local lines = {}
for _, entry in app.files.entries do
	table.insert(lines, entry) -- data, not elements
end
return ui.panel { grow = true, title = "Files",
	ui.list {
		id = "files",
		count = #lines,
		row_height = 24,
		row = function(i)
			local entry = lines[i]
			return ui.item { entry.name, selected = entry.path == state.selected, msg = { open = entry.path } }
		end,
	},
}
```

Rows must all be `row_height` tall (they are centered in it). The explorer
(`app/mods/explorer`) flattens its open folders this way.

**Views run only when needed.** The host keeps each view's last tree and
runs `view` again only when the mod's state or props may have changed (a
message reached its `update`), when a key of `app` it read changed (each
key counts on its own: a mod that reads `app.git` does not run when
`app.system` ticks), or on a reload. So read `app.x` where you use it;
`for k, v in app` reads every key and runs on any change. Views must not
depend on anything else (no clocks or random numbers that should change
between runs).

**`ui.memo`** keeps a costly part of a view while what it depends on stays
the same: `ui.memo("chart", { state.range, app.usage }, function() return
build_chart(...) end)`. Dependencies compare with `==`: values, or tables
that are replaced when they change (provider data is a new table each
time it changes; a state table changed in place stays the same table).

**Lazy panes.** `ui.pane { lazy = true, visible = state.right, ui.mount {
"changes", id = "side" } }` starts nothing until the pane first shows,
and while it is hidden again the instances in it sleep: their state stays,
but they neither run nor draw.

**Pinned badges.** `float = "top-right"` (or `{ at = "top-right", inset =
-4 }`) puts a count bubble on an icon without measuring anything.

**Shell with toggleable side panels.** Keep the visibility in the shell's
state, flip it from buttons, and pass it to `visible`:

```lua
ui.hsplit {
	ui.pane { size = 240, min = 160, max = 480, fixed = true, visible = state.left, surface = "sidebar", ui.region "left" },
	ui.pane { ui.region "main" },
	ui.pane { size = 280, fixed = true, visible = state.right, surface = "sidebar", ui.region "right" },
}
```

**Bottom panel.** Put a `ui.vsplit` inside the center pane, with a second
fixed pane (`size = 180`, `visible = state.bottom`) holding `ui.region "bottom"`.

**Activity bar switching the side pane, as in VS Code.** The shell keeps the
current view in its state and draws `use "activity-bar"` beside the split,
with the side pane holding `ui.region(state.activity)`. Each view is a
region name: its mods declare it (`region = "files"`), so a new view is a new
mod plus one entry in the bar's items. Hidden mods keep their state. Clicking
the current view folds the pane. Copy the whole shell from the component's
`usage` (`app/components/activity-bar.luau`). The starter's `app/main.luau`
has no activity bar on purpose; add one to an app's own shell (a profile),
not to the starter, unless the user asks.

**A mod that fills its pane.** The root should be `ui.panel { grow = true, ... }`.
Use `scroll = true` for a short scrolling body, and `ui.list` for lists that
can grow long (see "Long lists"). Several mods in one region stack. Give
`grow = true` only to those that should share the leftover height.

**Selectable list.** Use `ui.item { name, selected = name == state.selected, msg = { open = name } }`,
and in `update` set `state.selected` and send the choice on to whatever
shows it.

**Tabs.** Draw a row of small buttons in a panel's `actions`, with
`variant = state.tab == id and "primary" or "ghost"` and `msg = { tab = id }`,
then switch the body on `state.tab`.

**Toolbar or status bar.** Use `ui.row { padding = "small", align = "center", surface = "sidebar", border = "bottom", ... }`
with `ui.spacer()` between the left and right groups. For a status bar that
mods can extend, use `ui.region { name = "status", direction = "row" }`.

**Placing a mod explicitly.** `ui.mount "editor"` draws exactly that mod
wherever the shell puts it, whether or not it has a `region`.

**Restyling.** Change `style.luau`, not the modules: for example, make
panels flat with `panel = { radius = 0, margin = 0, border = "none", shadow = "none" }`.
For dark-only tweaks, give palette colors `{ light, dark }`. Give a mod its
own look with `styles` in the mod itself (see `app/mods/changes`). To
restyle an extension from the app side, add a rule for the same selector in
`style.luau`, which wins.

**Live data.** Mods can't read the machine (no `os`, `io` or network).
Providers share data under `app`: `app.system` (CPU, memory, network, from
the Wasm provider each second), `app.git` (branch, files, diffs, worktrees;
native, and it commits and pushes on request), and from process providers
`app.github` and `app.gitlab` (pull requests, checks, issues),
`app.agents` (coding agents running here), `app.files` (the project's
file tree), `app.docs` (native: reads and saves the project's text files
for a mod, answering it), `app.herdr` (whether herdr-core and tmux are
installed), and the native `app.acp` (coding agents a mod runs over ACP).
Their shapes are under "Built-in providers" below and typed in
`types/moonframe.d.luau` (`Git`, `Forge`, `Agents`, `Files`, `Herdr`, `Acp`,
and `Reply` for answers to actions). Views re-run with each publish. If a widget needs data
that isn't there, write a provider (below); don't fake it in Luau unless it
is labelled demo data, as in `mods/usage`.

**Providers: data Luau can't produce.** A provider is a folder
`providers/<name>/` with a `provider.toml`, in either folder. What it
publishes appears to every view as `app.<name>`, and a mod messages it with
`msg = ..., to = "provider:<name>"`. Adding, editing or removing a provider
folder starts, restarts or stops only that provider, while the window keeps
running; the others keep their state. A provider starts only once
something wants it: a drawn view reads `app.<name>` (a view that loops
over all of `app` wants every one), a mod subscribes to
`provider/<name>`, or a mod sends it a message; once started it keeps
running. `start = "always"` in provider.toml starts it with the app
instead, for one whose work matters before anything shows it. A failed provider shows a banner, and its error appears in the
terminal log. There are four kinds:

- **`kind = "wasm"`:** a WebAssembly component implementing
  `wit/provider.wit`, shipped as `provider.wasm` in the folder, or as the
  Rust crate it is built from (`Cargo.toml` with `crate-type = ["cdylib"]`
  and its own empty `[workspace]`, plus `src/lib.rs` using
  `wit_bindgen::generate!`).
  - It exports `start(settings)`, `tick()` every `interval` seconds, and
    `receive(message)`. All values are JSON text, and `tick` and `receive`
    return JSON to publish, or nothing.
  - With a `Cargo.toml` and Cargo installed (looked up on `PATH`, then in
    `~/.cargo/bin`), the host builds the crate when it starts and when its
    source changes, into `~/.cache/moonframe/providers`; a compile error
    becomes the provider's problem, never a quiet fallback to an older
    `provider.wasm`. Building needs the `wasm32-wasip2` target
    (`rustup target add wasm32-wasip2`). Otherwise it loads `provider.wasm`.
  - Installed apps ship `provider.wasm` only: `just stage` copies `app/`
    as the bundle holds it, each wasm provider prebuilt, without sources.
  - It runs sandboxed: no files, network or environment, a 64 MB memory cap,
    and an instruction budget per call.
  - It reaches the machine only through `capabilities`. Today that is
    `"system"`, which gives `host::system_load()`. Use this kind by default.
  - Only the running component is sandboxed. Building it runs Cargo and the
    crate's proc macros with the user's access. So never add a provider's
    source from an untrusted source without reading it, and keep its
    dependencies minimal. Something from elsewhere should come as
    `provider.wasm` alone.
    The host refuses to build a folder with a `build.rs` or a `.cargo/`.
  - `app/providers/system` is the example.
- **`kind = "static"`, `file = "sample.json"`:** a JSON file in the
  folder, published as it is: no code runs, so it needs no approval when
  installed. It is published again when the file changes and on any
  message (a mod's Refresh hears back). The file is checked like anything
  a provider publishes. Use it for sample data; `examples/tessel` does.
- **`kind = "process"`, `command = [...]`:** any program run in the folder.
  - Each line it prints on stdout is one JSON value to publish. Each message
    arrives on stdin as a `{"from":"<mod>","message":...}` line (compact,
    `from` first): check `from` before doing anything only one mod should
    ask for, as `providers/herdr` does before installing. Its stderr goes to the log.
  - It has the user's full access, so use it only when a sandbox can't reach
    the data, and say so in `provider.toml`.
  - It runs in its own folder (`providers/<name>/`); the built-in ones `cd`
    to the repository holding the app (`git -C ../.. rev-parse
    --show-toplevel`), or to `$LAYOUTS_REPO` when set.
  - `$MOONFRAME_PROJECT` is the project the user works in when it starts:
    the git repository of the terminal they last used. When that changes,
    the app sends `{"from":"","message":{"project":"<path>"}}`; an empty
    `from` is the app itself (no mod has one), so trust only that line.
  - If it exits, the provider fails (a banner) and stays stopped until its
    folder changes, so never exit on an error: publish
    `{"available":false,"reason":"..."}` and keep polling. Exit only when
    stdin closes. Under macOS's bash 3.2, `read -t` returns 1 for both a
    timeout and the end of input; tell them apart by elapsed time (see
    `github.sh`).
  - Keep caches out of the provider folder (a change there restarts it);
    use `~/.cache/moonframe/`.
  - The host refuses a line over 1 MiB or with over 10,000 values.
  - `app/providers/github` is the example; `app/providers/herdr` shows how
    to check who sent a message.
- **`kind = "native"`:** Rust compiled into the app, registered under the
  folder's name (`crates/moonframe/src/natives.rs`, each behind a cargo
  feature); a build without it shows why in the provider's banner. For
  what needs real libraries and the user's own access: git (on herdr-vcs),
  and later the forges and herdr-core.
  - It implements `moonframe_providers::Native` (`start`, `tick` every
    `interval`, `receive(from, message)`) and gets a `Handle` it may clone
    into its own threads, so a slow action (a push waiting on a hardware
    key) never holds up its ticks.
  - Besides publishing, it can **answer a mod**: `handle.send(mod, msg)`
    arrives in that mod's `update` like any message. Actions answer with
    `{ reply = <id the mod sent, else the action's name>, action, ok,
    message | error }`; keep what is waiting in the mod's state and clear
    it on the reply (`app/mods/status` does, for commit and push).
  - A panic stops that provider with a banner, never the app.
  - `crates/moonframe-git` is the example, with tests that drive it through
    a `Handle::new`.

**Built-in providers.** `git` is native (Rust in the app); `system` is
wasm; the rest are process providers in bash (3.2 or later), macOS and
Linux; on Windows they need a bash such as Git Bash, and `agents` does
not work there. Each publishes `available` and, when false,
only `reason` (with `repository = false` for git). Times are Unix seconds.

`app.git` (git): the repository is `LAYOUTS_REPO`, else the one of the
terminal the user last typed in (as `app.files`), else the one holding the
app. Every 5 s when `git status` or the numstat changed;
worktrees, branches, remote and stashes every 30 s or when HEAD moves.
Any message refreshes. `{ diff = "<path>" }` adds `diff` for that file and
keeps it fresh (only a file in `files`; any other path gets an empty diff);
`{ diff = false }` drops it. Writes, one at a time, only from the mods named
in `providers/git`'s `[settings] writers` (`status` as shipped; add yours
there), each answered to the mod that asked (`{ reply, action, ok, message | error }`):
`{ commit = "<message>", root, id = ... }` stages everything and commits;
`{ push = true, root, id = ... }` pushes the branch to `origin`, setting its
upstream. `root` is the repository the write is for, as `app.git.root` gave
it: the repository follows the terminal, so one that moved (or no `root`)
is refused, never written to another. Signing and pushing may wait on the user's hardware key: show
that something runs until the reply comes.

| field | shape |
|---|---|
| `repository`, `branch`, `changed`, `ahead`, `behind` | as before; `branch` is `"HEAD"` when `detached` |
| `root`, `detached`, `upstream` | top folder, boolean, `"origin/main"` or nil |
| `head` | `{ sha, short, subject, author, email, time }`, nil before a first commit |
| `added`, `deleted` | lines, working tree against HEAD (`git diff --numstat HEAD`) |
| `files` | `{ path, status, staged, unstaged, added, deleted, old_path? }`, at most 300 (`files_truncated`); `status` is `"M" "A" "D" "R" "?" "U"`; untracked files count 0 lines |
| `worktrees` | `{ path, branch, head, current, changed, ahead, behind, detached, locked, prunable, bare }` |
| `branches` | 20 newest local `{ name, head, time, upstream?, ahead, behind, gone, current }` |
| `remote` | `{ name, url, host, kind ("github" "gitlab" "other"), owner, repo, web_url? }` or nil |
| `stashes` | count |
| `diff` | `{ path, binary, lines, truncated, hunks = { { header, old_start, new_start, lines = { { kind ("add" "del" "ctx"), old?, new?, text } } } } }`, at most 1200 lines of 300 characters |

`app.docs` (docs, native): `{ available, root }`, the root `app.files`
lists, followed as it moves; a file read before it moved is still watched
and saved where it was read. `read` and `write` take `root` (the absolute
folder a file is in, as `app.files.path` named it) so a tab keeps its file
after the project moves; only a root the provider has had, or a project
the app has followed (kept across launches), is accepted. It reads and saves the project's text files (UTF-8, at most 1 MiB)
for the mod that asks, answering it (`{ reply, action, ok, path, ... }`):
`{ read = "<path>", id = ... }` answers with `text` and `version`, and from
then on sends that mod `{ changed = "<path>", version }` (or `missing =
true`) when the file changes on disk; `{ write = "<path>", text, version,
id = ... }` saves it atomically, keeping its permissions, and answers with
the new `version`, but refuses with `conflict = true` when the file
changed since `version` (or exists and none is given) unless `force =
true`; `{ close = "<path>" }` stops watching. Paths are relative to the
root, without `.`, `..` or `.git`, and must stay in it through links.
Anyone may read; only `[settings] writers` (`editor` as shipped) and the
components they draw may write. `use "markdown-editor"` is the whole
example.

A document can be long: `ui.markdown`'s `text` and a multiline
`ui.input`'s `value` take up to 1 MiB (other strings in a view, 16 KiB),
and a string in a message or in state up to 1 MiB (4 MiB of text per value).

`app.herdr` (herdr, from `bash`): `{ installed, version, path, tmux,
installing, error, fresh }`, every 30 s and on any message. The message
`"install"` runs herdr-core's own installer (into `~/.local/bin`) and sets
`fresh` once it has. Only the `herdr` mod's `"install"` counts: it sends
it when the user clicks Install on its offer. Whether that offer is up goes
out on the topic `offers/herdr` (a boolean), so another offer in the same
corner can wait for it, as `mods/skills` does.

`app.skills` (skills, from `bash`): `{ available, skills, target, status,
version, installing, error, fresh }`, every 60 s and on any message. The
agent skills the app ships (`skills/` beside the provider in a bundle, the
repository's `.claude/skills` from a checkout), against
`~/.config/moonframe/.claude/skills`: `status` is `"missing"`, `"stale"` or
`"current"`, `version` a checksum of the shipped ones. Only the `skills`
mod's `"install"` copies them, each skill folder replaced whole; it sends
that when the user agrees on its offer, which comes back when an update
ships newer skills.

`app.github` (github, via `gh`) and `app.gitlab` (gitlab, via `glab` and
`jq`, only for a GitLab remote): every 60 s, and on any message. Same shape;
GitLab merge requests fill `prs` and `current_pr`.

| field | shape |
|---|---|
| `repo` | `{ name_with_owner, url, default_branch }` |
| `viewer` | person: `{ login, name, bot, avatar_url, avatar_path }`; `avatar_path` is a cached file in `~/.cache/moonframe/images/avatars` (where `ui.image` may read it) once downloaded, else nil |
| `prs` | 30 open, newest first: `{ number, title, state ("open" "closed" "merged"), draft, url, branch, base, author (person), updated, review_decision, mergeable, merge_state, checks, tone, additions?, deletions? }` |
| `current_pr` | the same for the current branch plus `check_list = { { name, outcome, url } }`, or nil |
| `checks` | `{ total, passed, failed, pending, skipped }` (GitLab lists show the head pipeline only on `current_pr`) |
| `tone` | `"merged"`, `"closed"`, `"draft"`, `"attention"` (conflicts, failed checks, changes requested), `"waiting"` (checks running, review required), `"ready"`, `"held"` (blocked or behind) |
| `issues` | 30 open: `{ number, title, url, author, updated, labels = { { name, color } }, assignees = { login } }` |

`app.files` (files): the project's files, every 3 s when they changed. The
root is `MOONFRAME_FILES`, else the git repository of the terminal the user
last typed in (followed as they `cd` or switch tabs; a terminal outside any
repository leaves it where it was), else the git repository `moonframe` was
started in, else the one the app runs from; with none it is unavailable
(never home). `cwd` is where a terminal starts: that project, else the
launch folder, else the root. Only the root and the open folders are
listed, so a large folder costs nothing until it is opened: send
`{ open = "app\napp/mods" }`, the open folders' paths one per line, the
whole set each time (`mods/explorer` does). `.git` and `.DS_Store` are
never listed; paths with `..`, `.` or empty parts are refused, and so is a
folder whose real path (through symlinks) leaves the root.

| field | shape |
|---|---|
| `root`, `path` | the root folder's name and absolute path |
| `dirs` | `{ [path] = { { name, dir, ignored } } }`: the root at `""`, then each open folder; folders first, sorted without regard to case; `ignored` is git's |

`app.agents` (agents): every 4 s, from `ps` and the agents' folders (`/proc`
or `lsof`).

| field | shape |
|---|---|
| `agents` | `{ pid, ppid, name, command, cwd?, tty?, started, elapsed_seconds, cpu, status }`; `name` is `"claude" "codex" "gemini" "cursor" "opencode" "aider" "amp" "copilot" "droid" "kiro" "qwen" "goose" "crush" "auggie" "cline" "kimi" "grok"`; `status` is always `"running"` |
| `count` | how many |

`app.acp` (acp, native): coding agents run for mods over the Agent Client
Protocol (crates/moonframe-acp), each in the app's repository (else
`~/.config/moonframe`; `[settings] cwd` to change), one prompt per run,
published as it goes (every 0.25 s while something changed). Only mods in
`[settings] starters` (`feedback` as shipped) may start runs, and a run
obeys only the mod that started it. Agents are found as herdr-acp finds
them: `claude-agent-acp`, `codex-acp` (each else through `npx`),
`gemini --acp`, `opencode acp`, `copilot --acp`; `[settings] commands`
replaces one (`{ claude = ["my-acp"] }`). At most 4 run at once; finished
runs are kept (30) across launches.

Messages: `{ start = "<words>", id?, agent?, instructions?, context?,
}` (permission questions wait on the user, except those the
user's `[settings] allow` covers for the mod), `{ cancel = run }`, `{ forget = run }`
(finished ones), `{ answer = run, option = "<id>" | false }`.

| field | shape |
|---|---|
| `agents` | `{ name, title, installed, fetched }` (`fetched`: only `npx` can start it) |
| `agent`, `workdir`, `running` | the default agent, where agents work, how many runs go |
| `app` | the app folder the provider is in, from `workdir` (`app`, `examples/ledger`); nil when outside it |
| `pid` | this app's process, for an agent to screenshot the window (`just screenshot OUT PID`) |
| `runs` | newest first: `{ id, owner, agent, agent_title, text, thread?, status, started, finished?, error?, stop_reason?, message?, tools, tool_count, plan, question? }`; `status` is `"starting" "running" "waiting" "done" "failed" "cancelled" "stopped"`; `tools` the latest 12 `{ id, title, kind, status }`; `plan` `{ content, status }`; `question` `{ title, options = { { id, name, kind } } }` |

`[settings]` in `provider.toml` reaches `start`. `just check` validates the
manifests, and `just providers` builds and lints the Wasm ones. When a view
reads provider data, handle it being absent (`app.git` is nil until the
first publish).

To write one, copy the closest example folder, rename it, and change what it
publishes. Keep the published JSON small and shaped for the view (numbers
and short lists, not raw dumps), and pick the longest `interval` the widget
can live with. Wasm needs a host capability for anything outside its
sandbox; if none fits (today only `system`), use a process provider or add a
capability in `crates/moonframe-providers` (a Rust change). Reach for a
native provider only when it needs a Rust library (herdr-gpui's crates,
for one) or must answer actions; it is a Rust change and a restart. Write
it as a crate (`crates/moonframe-<name>`) implementing `Native`, register
it in `crates/moonframe/src/natives.rs` behind a cargo feature, add
`providers/<name>/provider.toml` with `kind = "native"`, and test it
through `Handle::new` as `crates/moonframe-git` does. Writes it does for
mods take an allowlist in `[settings]` (like git's `writers`), checked
against the sender, and answer every request, refused ones too.

**Progress bars and sparklines** are plain stacks: a fixed-`width` track
row holding a fill row of `width * fraction`, with `.track` and `.fill`
classes in the stylesheet. Sparkline bars are thin rows of varying `height`
in a row with `align = "end"`. See `mods/system` and `mods/usage`.

**Sheets and menus from a widget.** Wrap the widget in `ui.popover` and keep
`open` in the mod's state:

```lua
ui.popover {
	open = state.open, dismiss = "close", side = "top", width = 340,
	content = ui.column { ... },
	ui.row { class = "widget", msg = "toggle", ... },   -- the anchor
}
-- style.luau: popover = { background = "card", radius = 12, padding = 16,
--   shadow = "0 12px 32px -8px #0004", transition = "slide-fade fast ease-out" }
```

**Dialogs, palettes, sheets and toasts.** `ui.overlay` floats its
`content` over the whole window, not beside an anchor. Keep `open` in the
mod's state:

```lua
ui.overlay {
	open = state.palette, dismiss = "close", place = "top", backdrop = true, width = 520,
	keys = { enter = "run", up = "previous", down = "next", ["secondary-k"] = "close" },
	content = ui.column { ... },
}
-- style.luau: overlay = { background = "card", radius = 12, transition = "slide-fade fast" },
--   ["overlay-backdrop"] = { background = "#000000/50" }, [".sheet"] = { radius = 0 }
```

- `place`: `center` (default; dialogs), `top` (hangs from near the top,
  for palettes), `bottom` `left` `right` (sheets: the window's full width or
  height along that edge, sized by `height` or `width`), `top-start`
  `top-end` `bottom-start` `bottom-end` (toasts and notices, in a corner).
- `backdrop = true` dims the window behind and blocks it: clicks don't
  reach it, and a focused terminal there gets no keys. Clicking the
  backdrop, or outside the content when there is no backdrop, sends
  `dismiss` if given. With neither `backdrop` nor `dismiss` (a toast),
  clicks go through to the window around it.
- The content has the `overlay` part (and the overlay's `class` and
  `style`): `background`, `radius`, `border`, `padding`, `shadow`,
  `margin` (its distance from the window's edge; `top` hangs 80px down),
  `transition`. Without rules it is a bordered card on the background color.
- With a `slide` or `slide-fade` transition a sheet slides in from its edge
  (across its `width` or `height`), and a floating one rises a little into
  place. The backdrop fades with it.
- Open overlays stack in tree order: a later one is drawn over an earlier
  one, and a popover inside an overlay opens over it. Only the topmost open
  overlay or popover takes keys.
- In the Components browser an open overlay is drawn in place, as a card.

**Keys.** `keys = { name = msg, ... }` on `ui.overlay` and `ui.popover`:
while it is open and the topmost, each named keystroke sends its `msg` to
this mod's `update`, before anything else sees it (a focused terminal and
the app's shortcuts included). Plain Escape sends `dismiss` unless `keys`
has an `escape`. Other keys go on as usual.
- A name is modifiers and a key joined by `-`, as GPUI writes them:
  `escape`, `enter`, `shift-tab`, `ctrl-k`, `cmd-shift-p`, `cmd--` (minus).
- Modifiers: `ctrl`, `alt`, `shift`, `cmd` (⌘ on macOS, Super or the Windows
  key elsewhere) and `secondary` (⌘ on macOS, Ctrl on Linux and Windows).
  Prefer `secondary` for shortcuts so they feel native on each.
- Keys: `a`-`z`, `0`-`9`, a symbol such as `/` or `?` (a symbol matches the
  character typed, so `?` works whatever the layout), `f1`-`f24`, `escape`,
  `enter`, `tab`, `space`, `backspace`, `delete`, `insert`, `up`, `down`,
  `left`, `right`, `home`, `end`, `pageup`, `pagedown`. Names are lowercase:
  write `shift-a`, not `A`. A wrong name, or two names for the same key, is
  an error `just check` reports.
- Keys never take what is typed: a letter, digit, symbol or `space` needs
  `ctrl`, `alt`, `cmd` or `secondary` (`secondary-k`, `alt-?`), so an
  overlay cannot read keystrokes meant for a terminal behind it. For typing,
  use `ui.input` and its own `keys`.
- Modifiers must match exactly: `down` does not fire for `shift-down`.
- While the Inspector is picking or showing its card, Escape is its own.

**A new extension.** Create `app/mods/<name>/main.luau` with a `region` the
shell already draws. It appears without touching the shell. To give it a new
place, add a `ui.region "<new>"` to the shell. A region with no mods draws
nothing.

The shell in `app/main.luau` and the mods in `app/mods/` are working
examples of each pattern. Read them before larger changes.

## Extending the host

**In an app built on the library** (a crate whose `main` calls
`moonframe::Moonframe::new(...)`, with its own `app/` folder; see
`docs/library.md` in the Moonframe repository), Rust goes in the app's own
crate, never in Moonframe:
- a view Luau can't draw (an editor, a canvas, an existing GPUI view):
  a type implementing `moonframe::HostView` and `Render`, registered with
  `.view("editor", Editor::new)`, placed with `ui.view { name = "editor",
  id = "main", props = {...} }`. It is made on first draw, kept by mod and
  `id`, gets `set_props` when `props` change, and sends the mod's `update`
  messages with its `Messenger` (`ViewInit::messenger`). To await a tokio
  future from it, wrap it in `moonframe::on_runtime(future)` inside
  `cx.spawn`.
- data or actions: a `moonframe::Native` provider, `.native("index",
  Index::default)`, with `app/providers/index/provider.toml` holding
  `kind = "native"`. For async code (an HTTP client, anything on tokio),
  a `moonframe::AsyncNative` with `async fn start/tick/receive` instead,
  registered with `.native_async`; it runs on the app's runtime
  (`.tokio(handle)`, or the `#[tokio::main]` one), else one Moonframe makes.
There, `cargo run -- --check` checks the app (not `just check`), its
user folder is `~/.config/<name>`, and Moonframe's components, providers
and stylesheet sit under its `app/`, its mods shadowing none of theirs.
Rebuild and restart after Rust changes; Luau still reloads live.

**In Moonframe itself**, when a request needs something no element can
express (a tree, a chart), add an element in Rust. Say so to the user first: it needs a build and an
app restart, unlike everything above. `ui.icon`, `ui.terminal`, `ui.browser`
and `ui.input` are recent examples to copy; `ui.input` shows an element
whose host-side state is kept by mod and `id` and drawn with a `RenderOnce`
(`crates/moonframe/src/input.rs`, `render/input.rs`).

To add an icon, put its single-color 24×24 SVG (`currentColor`, no fixed
colors) in `assets/icons/`, credit its source in `assets/icons/LICENSES.md`,
and add its name to `ICONS` in `crates/moonframe-core/src/node.rs` (sorted)
and `IconName` in `types/moonframe.d.luau`. The binary embeds the folder and
looks there before gpui-kit's Lucide set; a test checks every name has a
file.

1. `crates/moonframe-core/src/node.rs`: add it to the `Element` tokens, and a
   struct and `Node` variant. Give it `look: Look` so classes, `style` and
   source lines reach it, and a `Part` in `crates/moonframe-core/src/style.rs`
   so the stylesheet can target it. Tokens of its own go in a `tokens!`.
2. `crates/moonframe-core/src/node/read.rs`: `impl Build` with its `KEYS`,
   and a `Read` type for any value that must be checked (as `IconName` and
   `TerminalId` are).
3. `crates/moonframe-core/src/prelude.luau`: its `ui.<name>` constructor,
   with the positional key if it has one.
4. `crates/moonframe/src/render/`: `impl Draw` for it (`elements.rs`, or
   a file of its own as `terminal.rs` is), and its arm in `draw_node` in
   `render.rs`.
5. `crates/moonframe/src/inspect.rs`: how the inspector names it
   (`describe`) and its line (`line`).
6. `types/moonframe.d.luau`: its props type and `ui.<name>`.
7. A test in `crates/moonframe-core/src/tests.rs`, then `just ci`.
8. This skill: the element table, the selector list, and a recipe if it is
   something agents will reach for. A component wrapping it makes it show up
   in the Components menu.

Anything that runs code or touches the machine belongs in a provider or
behind a host element that the mod can only parameterise (as terminals take
a `cwd`, never a command, and browsers an address, never a script). The Luau sandbox is the boundary; keep it.

## Whole apps

A whole app (its own shell, menus, icon, bundle) runs as a profile beside the
starter: see the **moonframe-apps** skill (`.claude/skills/moonframe-apps`)
for `moonframe.toml`, `moonframe app new | link | install`, `--profile`,
menus and About, release mode, `just bundle-app`, and the perf log.

## Sharing mods

A package is a git repository, in one of two shapes:
- **one mod**: `main.luau` at the root. The mod is named after the
  repository, lowercased, with other characters as `-` (`My.Clock` is
  `my-clock`).
- **an app folder**: any of `mods/`, `providers/`, `components/`,
  `style.luau`, laid out as above. No `main.luau` at the root (that would
  make it one mod), and so no shell.

```sh
moonframe mod install github.com/owner/repo   # or owner/repo, a git URL, a folder; @ref pins it
moonframe mod list | update [NAME] | remove NAME | enable NAME | disable NAME
moonframe mod new ~/code/my-mod                # a git repo with one mod and a README
moonframe mod link ~/code/my-mod               # use a folder in place, to develop it live
```

Installs clone into `~/.config/moonframe/packages/<host>/<owner>/<repo>`
and are listed in `~/.config/moonframe/packages.toml`; an open window
picks up every change. Mods, components and styles are sandboxed Luau;
providers that run as the user (process providers, Wasm, which is
built from source, and native ones, which replace a built-in provider
with the package's settings) are shown and must be approved, or passed
`--yes`.
**Packages ship source only**: install, update and link refuse a package
holding a `provider.wasm` or any other file that is not text (images
aside), so the user, or an agent, can review everything it runs before
installing. A Wasm provider in a package ships its crate (`Cargo.toml`,
`src/`) and never a build; gitignore `target/`. When asked whether a
package is safe, read its providers' source and commands, not just the
mods. A package's `kind = "native"` provider replaces the built-in one
of that name with its own `[settings]`: a `providers/git` widening git's
`writers` lets that package's mods commit and push, so read those too.
Likewise a package's mod named like a writer (`mods/status`) takes the
user's place and its right to commit and push; install names it and asks.
Remote URLs reach mods without the credentials they may carry. To publish, push the
repository anywhere git can clone from and share the install line. When
the user asks to share something they built in `~/.config/moonframe`,
copy it into a new package (`mod new`), `link` it, and remove the original
so the two do not shadow each other. Don't edit installed packages in
`packages/`: an update replaces them; copy the mod into the user's `mods/`
instead.

## Commands

| Command | Does |
| --- | --- |
| `just check` | loads everything headlessly (your folder and packages included); run after each edit |
| `just check-builtin` | the same for `app/` alone |
| `just types` | strict Luau type check against `types/moonframe.d.luau` |
| `just screenshot OUT PID` | captures the window of the process you started |
| `just component-shot NAME [OUT]` | captures a component's preview page from a background instance it starts and stops; never takes focus |
| `just run` | opens the window on `app/` (the user usually has one open) |
| `just providers` | builds and lints the Wasm providers |
| `just stage` | stages `app/` as the bundle ships it, Wasm prebuilt (icons are embedded in the binary, so nothing to stage) |
| `just bundle` | builds `target/bundle/Moonframe.app` (binary, staged app, icon) |
| `just install` | builds the app and installs it in `/Applications`; use it to try mods in `~/.config/moonframe` against the app as users get it |
| `just ci` | format check, lint, tests, types, providers, check |
