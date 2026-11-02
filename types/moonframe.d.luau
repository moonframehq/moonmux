-- Types for Moonframe modules: the `ui`, `mod`, `style`, and `log`
-- globals the host gives every script. `just types` checks app/ against
-- them in strict mode; editors with luau-lsp read them for completion.
--
-- These mirror what the host accepts (crates/moonframe-core). The host still
-- checks every value at run time; types catch mistakes before it runs.

-- Values that cross between a module and the host: messages and state.
type Data = nil | boolean | number | string | { [any]: any }

type Space = "none" | "small" | "normal" | "large"
type Tone = "normal" | "muted" | "accent" | "success" | "warning" | "danger"
type TextSize = "small" | "normal" | "large" | "title" | "headline" | "display" | "hero"
type Weight = "light" | "normal" | "medium" | "semibold" | "bold"
type Align = "stretch" | "start" | "center" | "end"
type Justify = "start" | "center" | "end" | "between"
type Surface = "background" | "sidebar" | "muted" | "secondary"
type Edge = "none" | "all" | "top" | "bottom" | "left" | "right"
type Border = boolean | Edge
type Direction = "row" | "column"
type Variant = "normal" | "primary" | "ghost" | "outline" | "danger"
type Command = "quit" | "reload" | "toggle-theme" | "show-themes" | "inspect" | "minimize" | "maximize" | "close-window" | "check-for-updates" | BrowserCommand
-- Run on the `ui.browser` the control's `browser` key names.
type BrowserCommand = "browser-back" | "browser-forward" | "browser-reload" | "browser-stop" | "browser-open"

-- `#rrggbb`, a palette name, or a theme color, optionally `name/NN`.
type Color = string
-- A space name or pixels.
type Spacing = Space | number
-- One spacing, `{ y, x }`, `{ top, right, bottom, left }`, or named edges.
type Sides = Spacing | { Spacing } | {
	x: Spacing?,
	y: Spacing?,
	top: Spacing?,
	right: Spacing?,
	bottom: Spacing?,
	left: Spacing?,
}
type ShadowLayer = {
	x: number?,
	y: number?,
	blur: number?,
	spread: number?,
	color: Color?,
	inset: boolean?,
}
-- A preset (`"small"`), a CSS string (`"0 4px 12px #0003"`), a layer, or layers.
type Shadow = string | ShadowLayer | { string | ShadowLayer }
type TransitionKind = "slide" | "fade" | "slide-fade" | "none"
-- `"slide 220ms ease-out"`, or a table; durations are ms, `"220ms"`, or a speed.
type Transition = string | {
	kind: TransitionKind?,
	duration: (number | string)?,
	curve: string?,
	-- Wait before moving, as CSS's transition-delay.
	delay: (number | string)?,
}

type Style = {
	background: Color?,
	-- A linear gradient over the background: `"180deg, #ffa53a, #ff6200"`
	-- (0 runs up, 90 right, 180 down, the default), or `{ from, to, angle }`.
	gradient: (string | { from: Color, to: Color, angle: number? })?,
	color: Color?,
	border: Border?,
	border_color: Color?,
	-- One radius, or each corner's clockwise from the top left as in CSS:
	-- { 8, 8, 0, 0 } rounds the top only.
	radius: (number | { number })?,
	padding: Sides?,
	gap: Spacing?,
	margin: Sides?,
	shadow: Shadow?,
	size: TextSize?,
	weight: Weight?,
	font: ("sans" | "mono")?,
	-- Letter case of a text: `upper` for headings.
	case: ("upper" | "lower" | "none")?,
	opacity: number?,
	-- Pixels, under the element's own `width` and `height`: what a named
	-- state's rule can change (`[".drawer:closed"] = { width = 48 }`).
	width: number?,
	height: number?,
	-- How a pane, popover or overlay shows and hides, a keyed child arrives
	-- and goes, and an element with a `state` moves to its next state's look
	-- (colors, opacity, width and height ease between the two).
	transition: Transition?,
	-- Replaces the transition's delay (ms, or "40ms"): set from an item's
	-- index so a list's items move one after another (a staggered fade).
	delay: (number | string)?,
	-- Hide what overflows the box instead of drawing it outside.
	clip: boolean?,
}

-- What a `ui` function returns; the host reads its fields.
type Element = { type: string }
-- A child: an element, a string (plain text), or false/nil (skipped).
type Child = Element | string | false | nil
-- Class names: `"a b"` or `{ "a", "b" }`.
type Class = string | { string }

-- Where `float` pins an element over its parent.
type FloatAt =
	"top-left"
	| "top-right"
	| "bottom-left"
	| "bottom-right"
	| "top"
	| "bottom"
	| "left"
	| "right"
	| "fill"
-- Out of the flow, over the parent: pinned to an edge or corner
-- (`"top-right"`), set in from it (`{ at = "top-right", inset = 4 }`), or at
-- pixel offsets from its edges (`{ top = -4, right = -4 }`, negative overhangs).
type Float = FloatAt | { at: FloatAt, inset: number? } | {
	top: number?,
	right: number?,
	bottom: number?,
	left: number?,
}

-- Each props type is spelled out rather than combined with `&`, so string
-- literals such as `variant = "ghost"` check against their unions.

-- A stack with `msg` or `command` is clickable as a whole, like a button.
type StackProps = {
	class: Class?,
	style: Style?,
	-- A state of the view's own naming, which `:<state>` rules match.
	state: string?,
	msg: Data,
	to: string?,
	command: Command?,
	-- With a browser command: the `id` of the `ui.browser` (of the mod `to`, else this one).
	browser: string?,
	gap: Space?,
	padding: Space?,
	grow: boolean?,
	width: number?,
	height: number?,
	surface: Surface?,
	border: Border?,
	-- `true` scrolls along its axis; or "x", "y", "both".
	scroll: (boolean | "x" | "y" | "both")?,
	-- false keeps the content's size when the parent is short of space.
	shrink: boolean?,
	-- Where it sits across the parent's axis, over the parent's `align`.
	self_align: Align?,
	-- Keep its own width in a column; follow a row's `align` as usual.
	hug: boolean?,
	max_width: number?,
	max_height: number?,
	-- Out of the flow, over the parent: see `Float`.
	float: Float?,
	align: Align?,
	justify: Justify?,
	wrap: boolean?,
	-- Matches `:selected` rules, as on an item: tabs, segments, tiles.
	selected: boolean?,
	-- Extras: shown under the pointer; sent on a right-click; copied on a click.
	tooltip: string?,
	context: Data,
	copy: string?,
	-- Moves the window when dragged and zooms it on a double click, like a
	-- title bar; buttons and clickable stacks inside still take clicks.
	drag: boolean?,
	-- Names it among its siblings (1 to 64 of a-z, 0-9, - and _), so it stays
	-- itself as they come and go: with a `transition` rule it grows in when
	-- it appears and shrinks away when it goes, as tabs open and close.
	key: string?,
	-- Opens the mod named this in a window of its own (or raises it) when
	-- clicked: a tool that stays open beside the main window. Takes no `msg`.
	window: string?,
	[number]: Child,
}

type PanelProps = {
	class: Class?,
	style: Style?,
	state: string?,
	gap: Space?,
	padding: Space?,
	grow: boolean?,
	width: number?,
	height: number?,
	surface: Surface?,
	border: Border?,
	-- `true` scrolls along its axis; or "x", "y", "both".
	scroll: (boolean | "x" | "y" | "both")?,
	-- false keeps the content's size when the parent is short of space.
	shrink: boolean?,
	-- Where it sits across the parent's axis, over the parent's `align`.
	self_align: Align?,
	-- Keep its own width in a column; follow a row's `align` as usual.
	hug: boolean?,
	max_width: number?,
	max_height: number?,
	-- Out of the flow, over the parent: see `Float`.
	float: Float?,
	title: string?,
	actions: { Child }?,
	[number]: Child,
}

type PaneProps = {
	class: Class?,
	style: Style?,
	state: string?,
	size: number?,
	min: number?,
	max: number?,
	fixed: boolean?,
	visible: boolean?,
	-- Start what it mounts only once it first shows, and keep it asleep
	-- (neither drawn nor run) while it is hidden.
	lazy: boolean?,
	gap: Space?,
	padding: Space?,
	surface: Surface?,
	-- `true` scrolls along its axis; or "x", "y", "both".
	scroll: (boolean | "x" | "y" | "both")?,
	[number]: Child,
}

type SplitProps = {
	id: string?,
	-- false: no hairline between panes; a handle shows only when hovered.
	divider: boolean?,
	[number]: Element | false | nil,
}

type TextProps = {
	class: Class?,
	style: Style?,
	state: string?,
	text: string?,
	tone: Tone?,
	size: TextSize?,
	weight: Weight?,
	truncate: boolean?,
	-- Wrap to at most this many lines, then an ellipsis.
	lines: number?,
	-- false keeps each line whole (code in a container that scrolls sideways).
	wrap: boolean?,
	-- A line through it: a task done.
	strike: boolean?,
	-- Shown under the pointer; sent on a right-click (to the mod `to`, else
	-- this one); copied on a click.
	tooltip: string?,
	context: Data,
	copy: string?,
	to: string?,
	[number]: string,
}

-- Clicking sends `msg` to an `update` (to the mod `to`, else its own), or
-- runs an app `command`.
type ButtonProps = {
	class: Class?,
	style: Style?,
	state: string?,
	msg: Data,
	to: string?,
	command: Command?,
	-- With a browser command: the `id` of the `ui.browser` (of the mod `to`, else this one).
	browser: string?,
	label: string?,
	-- Drawn before the label; with no label, an icon-only button.
	icon: IconName?,
	variant: Variant?,
	small: boolean?,
	disabled: boolean?,
	tooltip: string?,
	context: Data,
	copy: string?,
	[number]: string,
}

type ItemProps = {
	class: Class?,
	style: Style?,
	state: string?,
	msg: Data,
	to: string?,
	command: Command?,
	-- With a browser command: the `id` of the `ui.browser` (of the mod `to`, else this one).
	browser: string?,
	label: string?,
	detail: string?,
	badge: string?,
	selected: boolean?,
	tooltip: string?,
	context: Data,
	copy: string?,
	[number]: string,
}

type BadgeProps = {
	class: Class?,
	style: Style?,
	state: string?,
	text: string?,
	tone: Tone?,
	-- Shown under the pointer; sent on a right-click (to the mod `to`, else
	-- this one); copied on a click.
	tooltip: string?,
	context: Data,
	copy: string?,
	to: string?,
	[number]: string,
}

-- The bundled icons (Lucide); the host refuses any other name.
type IconName =
	| "a-large-small"
	| "activity"
	| "agent-amp"
	| "agent-claude"
	| "agent-codex"
	| "agent-copilot"
	| "agent-cursor"
	| "agent-droid"
	| "agent-gemini"
	| "agent-grok"
	| "agent-kiro"
	| "agent-opencode"
	| "agent-qwen"
	| "archive"
	| "arrow-down"
	| "arrow-left"
	| "arrow-right"
	| "arrow-up"
	| "arrow-up-down"
	| "arrow-up-right"
	| "asterisk"
	| "at-sign"
	| "ban"
	| "battery"
	| "battery-charging"
	| "battery-full"
	| "battery-low"
	| "battery-medium"
	| "battery-warning"
	| "bell"
	| "bell-off"
	| "book-open"
	| "bookmark"
	| "bot"
	| "brand-anthropic"
	| "brand-github"
	| "brand-gitlab"
	| "brand-mistral"
	| "brand-ollama"
	| "brand-openai"
	| "bug"
	| "building-2"
	| "calendar"
	| "calendar-days"
	| "case-sensitive"
	| "chart-column"
	| "chart-line"
	| "chart-pie"
	| "check"
	| "chevron-down"
	| "chevron-left"
	| "chevron-right"
	| "chevron-up"
	| "chevrons-left"
	| "chevrons-right"
	| "chevrons-up-down"
	| "circle"
	| "circle-alert"
	| "circle-check"
	| "circle-dashed"
	| "circle-dot"
	| "circle-help"
	| "circle-pause"
	| "circle-play"
	| "circle-question-mark"
	| "circle-stop"
	| "circle-user"
	| "circle-x"
	| "clock"
	| "close"
	| "cloud"
	| "cloud-off"
	| "code"
	| "coffee"
	| "columns-2"
	| "command"
	| "copy"
	| "cpu"
	| "dash"
	| "database"
	| "delete"
	| "dot"
	| "download"
	| "ellipsis"
	| "ellipsis-vertical"
	| "external-link"
	| "eye"
	| "eye-off"
	| "file"
	| "file-archive"
	| "file-text"
	| "filter"
	| "flag"
	| "folder"
	| "folder-closed"
	| "folder-git-2"
	| "folder-open"
	| "frame"
	| "funnel"
	| "gallery-vertical-end"
	| "git-branch"
	| "git-commit-horizontal"
	| "git-compare"
	| "git-merge"
	| "git-pull-request"
	| "git-pull-request-closed"
	| "git-pull-request-draft"
	| "github"
	| "gitlab"
	| "globe"
	| "grip-vertical"
	| "hard-drive"
	| "hash"
	| "headphones"
	| "heart"
	| "heart-off"
	| "history"
	| "house"
	| "image"
	| "inbox"
	| "info"
	| "inspector"
	| "key-round"
	| "keyboard"
	| "laptop"
	| "layers"
	| "layout-dashboard"
	| "layout-grid"
	| "link"
	| "list"
	| "list-checks"
	| "list-tree"
	| "loader"
	| "loader-circle"
	| "lock"
	| "lock-open"
	| "log-out"
	| "mail"
	| "mail-open"
	| "mails"
	| "map"
	| "maximize"
	| "maximize-2"
	| "memory-stick"
	| "menu"
	| "message-square"
	| "messages-square"
	| "mic"
	| "minimize"
	| "minimize-2"
	| "minus"
	| "monitor"
	| "moon"
	| "network"
	| "octagon-alert"
	| "palette"
	| "panel-bottom"
	| "panel-bottom-open"
	| "panel-left"
	| "panel-left-close"
	| "panel-left-open"
	| "panel-right"
	| "panel-right-close"
	| "panel-right-open"
	| "paperclip"
	| "pause"
	| "pencil"
	| "pin"
	| "pin-off"
	| "play"
	| "plug"
	| "plus"
	| "power"
	| "redo"
	| "redo-2"
	| "refresh-ccw"
	| "refresh-cw"
	| "replace"
	| "reply"
	| "resize-corner"
	| "rocket"
	| "rotate-ccw-clock"
	| "rotate-cw"
	| "rows-2"
	| "search"
	| "send"
	| "server"
	| "settings"
	| "settings-2"
	| "shield"
	| "sort-ascending"
	| "sort-descending"
	| "sparkles"
	| "split-square-horizontal"
	| "split-square-vertical"
	| "square"
	| "square-check"
	| "square-pen"
	| "square-split-horizontal"
	| "square-split-vertical"
	| "square-terminal"
	| "star"
	| "star-fill"
	| "star-off"
	| "sun"
	| "tag"
	| "tags"
	| "terminal"
	| "thumbs-down"
	| "thumbs-up"
	| "trash"
	| "trash-2"
	| "triangle-alert"
	| "undo"
	| "undo-2"
	| "unlock"
	| "upload"
	| "user"
	| "user-plus"
	| "users"
	| "wand-sparkles"
	| "wifi"
	| "wifi-off"
	| "window-close"
	| "window-maximize"
	| "window-minimize"
	| "window-restore"
	| "workflow"
	| "zap"

-- Drawn at the size and color of text, so `color` and `size` style it.
type IconProps = {
	class: Class?,
	style: Style?,
	state: string?,
	name: IconName?,
	tone: Tone?,
	size: TextSize?,
	-- Turns without end, for a loader (`loader-circle`); `animation_speed`
	-- paces it and 0 stops it.
	spin: boolean?,
	-- Shown under the pointer; sent on a right-click (to the mod `to`, else
	-- this one); copied on a click.
	tooltip: string?,
	context: Data,
	copy: string?,
	to: string?,
	-- Luau doesn't check literals in list positions against a union, so
	-- write `name = "cpu"` (checked here) rather than `{ "cpu", ... }`.
	[number]: string,
}

type Fit = "contain" | "cover" | "fill" | "scale-down" | "none"

-- A picture from a file: relative to the mod's folder, `~/...`, or absolute.
-- Not a URL: have a provider download it to a cache. A missing file leaves
-- an empty box of `width` x `height`. `radius` in a rule rounds it.
type ImageProps = {
	class: Class?,
	style: Style?,
	state: string?,
	src: string?,
	width: number?,
	height: number?,
	fit: Fit?,
	-- Shown under the pointer; sent on a right-click (to the mod `to`, else
	-- this one); copied on a click.
	tooltip: string?,
	context: Data,
	copy: string?,
	to: string?,
	[number]: string,
}

-- A line through `points`, evenly spaced across its box. Its style's
-- `color` strokes it, `background` fills the area to the baseline, and
-- `border_color` draws the baseline, dashed.
type LineProps = {
	class: Class?,
	style: Style?,
	points: { number },
	min: number?,
	max: number?,
	baseline: number?,
	area: boolean?,
	dots: ("last" | "all")?,
	stroke: number?,
	dashed: boolean?,
	width: number?,
	height: number?,
}

-- A box of diagonal stripes in its style's `color` over its `background`.
type HatchProps = {
	class: Class?,
	style: Style?,
	gap: number?,
	width: number?,
	height: number?,
	grow: boolean?,
}

-- Markdown the host renders: headings, emphasis, lists, task lists, tables,
-- quotes and highlighted code, up to 1 MiB. The `markdown` rule styles the
-- box, `markdown-code` its code and `markdown-quote` its quotes. A link never
-- opens by itself: a click sends `link`, a table getting a `url` field and
-- anything else arriving as `{ msg = link, url = url }`. Pictures are not
-- loaded.
type MarkdownProps = {
	class: Class?,
	style: Style?,
	state: string?,
	text: string?,
	link: Data,
	-- The mod `link` goes to; this one when unset.
	to: string?,
	grow: boolean?,
	-- Scroll its own content; give it `grow` or a `height`.
	scroll: boolean?,
	width: number?,
	height: number?,
	max_width: number?,
	[number]: string,
}

type TerminalEngine = "alacritty" | "ghostty"
-- What keeps the shell running: `auto` (default) is herdr-core, else tmux,
-- else a pty. herdr-core and tmux keep it across launches; a pty ends with
-- the app. One that isn't installed falls back to the next.
type TerminalBackend = "auto" | "herdr-core" | "tmux" | "pty"

-- A live shell (your login shell). The host keeps it by mod and `id`.
type TerminalProps = {
	class: Class?,
	style: Style?,
	state: string?,
	id: string?,
	-- Where it starts: a path or `~/...`; the home folder when unset.
	cwd: string?,
	engine: TerminalEngine?,
	backend: TerminalBackend?,
	[number]: string,
}

-- A web page in a native view, kept by mod and `id`. `url` navigates when
-- it changes from the last one given; the user's own navigation stands
-- until then. Only http, https and file addresses.
type BrowserProps = {
	class: Class?,
	style: Style?,
	state: string?,
	id: string?,
	url: string?,
	-- The page's zoom factor, 0.25 to 5.
	zoom: number?,
	-- Sent as the page navigates, its title changes or a load starts or
	-- ends, with `url`, `title` and `loading` added, and `back` and
	-- `forward` (whether it can go there) where the platform says (a
	-- non-table message arrives as `{ msg = ... }`).
	navigate: Data,
	-- Sent as a load starts and finishes, with `loading` and `url` added.
	loading: Data,
	[number]: string,
}

-- A view the app writes in Rust and registers by `name` (an app on the
-- Moonframe library; the starter has none). The host keeps it by mod and
-- `id`, so it lives across reloads and while hidden, and hands it `props`,
-- telling it when they change. What the user does in it comes back to this
-- mod's `update` as messages the view chooses.
type ViewProps = {
	class: Class?,
	style: Style?,
	state: string?,
	name: string?,
	id: string?,
	props: Data,
	[number]: string,
}

-- A text field. The host keeps its text by mod and `id` (else its place in
-- the tree); `value` replaces the text only when it differs from the last
-- `value` passed. Messages carry the text: a table `msg` gets a `value`
-- field, anything else arrives as `{ msg = msg, value = text }`.
type InputProps = {
	class: Class?,
	style: Style?,
	state: string?,
	id: string?,
	value: string?,
	placeholder: string?,
	-- Sent on each edit.
	change: Data,
	-- Sent on Enter, or ⌘/Ctrl-Enter when `multiline`.
	submit: Data,
	-- While focused, these keys send their message instead of editing:
	-- `{ up = "prev", escape = "close", ["shift-tab"] = "back",
	-- ["secondary-k"] = "clear" }` (`secondary` is ⌘ on macOS, Ctrl elsewhere).
	keys: { [string]: any }?,
	-- The mod the messages go to; the drawing mod when unset.
	to: string?,
	multiline: boolean?,
	-- Visible lines of a multiline field (default 3); it scrolls past them.
	rows: number?,
	disabled: boolean?,
	-- Take keyboard focus when it appears, or when this turns true.
	focus: boolean?,
	-- The monospace font.
	mono: boolean?,
	width: number?,
	grow: boolean?,
	-- Make it the address bar of this mod's `ui.browser` with this id (or
	-- `to`'s): Enter loads the text (a host over https, words as a web
	-- search), focus selects it, and Escape puts back the page's address.
	browser: string?,
}

type RegionProps = {
	name: string,
	direction: Direction?,
	gap: Space?,
	justify: Justify?,
}

-- Which side of its anchor a popover opens on.
type Side = "top" | "bottom" | "left" | "right"
-- The anchor's edge it lines up with: left or right above and below, top or
-- bottom beside.
type Anchor = "start" | "end"

-- Keystrokes an open popover or overlay takes, to the message each sends
-- to this mod's `update`: `{ escape = "close", down = "next", ["cmd-k"] = "find" }`.
-- A name is modifiers then a key, joined by `-`: modifiers `ctrl`, `alt`,
-- `shift`, `cmd` (Super/Windows elsewhere) and `secondary` (cmd on macOS,
-- ctrl elsewhere); keys `a`-`z`, `0`-`9`, a symbol such as `/`, `f1`-`f24`,
-- `escape`, `enter`, `tab`, `space`, `backspace`, `delete`, `insert`, `up`,
-- `down`, `left`, `right`, `home`, `end`, `pageup`, `pagedown`.
type Keys = { [string]: Data }

-- Content floating beside its anchor (the list part). While `open`, a click
-- outside sends `dismiss`. A `popover` stylesheet rule styles and animates it.
type PopoverProps = {
	class: Class?,
	style: Style?,
	state: string?,
	open: boolean?,
	dismiss: Data,
	-- The mod `dismiss` and `keys` messages go to, if not this one.
	to: string?,
	side: Side?,
	align: Anchor?,
	gap: number?,
	width: number?,
	keys: Keys?,
	content: Element,
	[number]: Child,
}

type Placement =
	"center"
	| "top"
	| "bottom"
	| "left"
	| "right"
	| "top-start"
	| "top-end"
	| "bottom-start"
	| "bottom-end"

-- Content floating over the whole window: a dialog or palette (`center`,
-- `top`), a sheet along an edge (`bottom`, `left`, `right`), a toast in a
-- corner. Open overlays stack in tree order above popovers. A click on the
-- backdrop or outside the content sends `dismiss`, and so does Escape unless
-- `keys` takes it. An `overlay` rule styles and animates the content, an
-- `overlay-backdrop` rule the dimmed window behind.
type OverlayProps = {
	class: Class?,
	style: Style?,
	state: string?,
	open: boolean?,
	dismiss: Data,
	-- The mod `dismiss` and `keys` messages go to, if not this one.
	to: string?,
	place: Placement?,
	backdrop: boolean?,
	width: number?,
	height: number?,
	keys: Keys?,
	content: Element,
}

-- Rows of one height, of which only those showing are built and drawn, so
-- a list of any length costs a screenful. `row(i)` builds row `i` (from 1)
-- and is called again as the list scrolls; keep it cheap and pure.
type ListProps = {
	class: Class?,
	style: Style?,
	state: string?,
	-- Names it among this mod's lists (default "list"); two need two ids.
	id: string?,
	count: number,
	-- Each row's height in pixels.
	row_height: number,
	row: (index: number) -> Child,
	gap: Space?,
	padding: Space?,
	grow: boolean?,
	width: number?,
	height: number?,
	surface: Surface?,
	border: Border?,
	shrink: boolean?,
	self_align: Align?,
	max_width: number?,
	max_height: number?,
	float: Float?,
}

declare ui: {
	row: (props: StackProps) -> Element,
	column: (props: StackProps) -> Element,
	panel: (props: PanelProps) -> Element,
	hsplit: (props: SplitProps) -> Element,
	vsplit: (props: SplitProps) -> Element,
	pane: (props: PaneProps) -> Element,
	text: (props: string | TextProps) -> Element,
	button: (props: ButtonProps) -> Element,
	item: (props: ItemProps) -> Element,
	badge: (props: string | BadgeProps) -> Element,
	icon: (props: IconName | IconProps) -> Element,
	image: (props: string | ImageProps) -> Element,
	line: (props: LineProps) -> Element,
	hatch: (props: HatchProps) -> Element,
	terminal: (props: string | TerminalProps) -> Element,
	browser: (props: string | BrowserProps) -> Element,
	view: (props: string | ViewProps) -> Element,
	input: (props: InputProps) -> Element,
	markdown: (props: string | MarkdownProps) -> Element,
	divider: () -> Element,
	spacer: () -> Element,
	region: (props: string | RegionProps) -> Element,
	-- With an `id`, an instance of the mod with state of its own.
	mount: (name: string | MountProps) -> Element,
	popover: (props: PopoverProps) -> Element,
	overlay: (props: OverlayProps) -> Element,
	list: (props: ListProps) -> Element,
	-- What `build()` returned the last time this view asked for `key` with
	-- the same `deps` (compared with `==`), or `build()` now.
	memo: <T>(key: string, deps: { any }, build: () -> T) -> T,
}

-- This machine's load, sampled by the host once a second.
type System = {
	-- Fractions from 0 to 1.
	cpu: number,
	-- Oldest first, ending with `cpu`.
	cpu_history: { number },
	memory: number,
	memory_used: number,
	memory_total: number,
	-- Bytes per second.
	received: number,
	transmitted: number,
}

type Platform = "macos" | "linux" | "windows"
type Titlebar = "system" | "hidden"

-- The window, for a title bar the layout draws itself.
type WindowState = {
	platform: Platform,
	-- `hidden` when the shell asked for it and the system agreed (a Linux
	-- compositor can insist on its own title bar).
	titlebar: Titlebar,
	maximized: boolean,
	fullscreen: boolean,
	-- The frontmost window.
	active: boolean,
}

-- Light, dark, or as the system is (switching as it does).
type AppearanceMode = "system" | "light" | "dark"

-- `app.appearance`: set it with `{ to = "app:appearance", msg = "dark" }`.
type Appearance = {
	-- What the user chose, kept across launches for the app and profile.
	mode: AppearanceMode,
	-- Whether the window is dark now (as `app.dark`).
	dark: boolean,
	-- The theme showing: `light_theme` or `dark_theme`, as the window is.
	theme: string,
	-- The theme light windows show and the one dark windows show, kept
	-- across launches; set with `{ to = "app:theme", msg = { dark = "Nord" } }`.
	-- "Default" is the stock look with the stylesheet's colors.
	light_theme: string,
	dark_theme: string,
	-- Every theme, for a picker of the app's own: "Default" first, once for
	-- light and once for dark, then the rest by name.
	themes: { ThemeInfo },
}

-- A theme as `app.appearance.themes` lists it. Colors are `#rrggbb`.
type ThemeInfo = {
	name: string,
	-- Made for dark windows: only such a theme can be the `dark_theme`.
	dark: boolean,
	background: string,
	foreground: string,
	accent: string,
	-- Six of its colors: red, green, yellow, blue, magenta, cyan.
	colors: { string },
}


-- `app.git`, from providers/git. When `available` is false only `reason`
-- is set (and `repository = false`); every other field is there otherwise.
type GitFile = {
	path: string,
	-- "M", "A", "D", "R" (renamed or copied), "?" (untracked) or "U" (conflict).
	status: string,
	staged: boolean,
	unstaged: boolean,
	-- Lines against HEAD; 0 for binary and untracked files.
	added: number,
	deleted: number,
	-- Renames only.
	old_path: string?,
}

type GitWorktree = {
	path: string,
	-- "" when detached.
	branch: string,
	head: string,
	current: boolean,
	-- Files with changes.
	changed: number,
	ahead: number,
	behind: number,
	detached: boolean,
	locked: boolean,
	prunable: boolean,
	bare: boolean,
}

type GitBranch = {
	name: string,
	-- Short sha.
	head: string,
	-- Unix seconds of its last commit.
	time: number,
	upstream: string?,
	ahead: number,
	behind: number,
	-- The upstream was deleted.
	gone: boolean,
	current: boolean,
}

type GitRemote = {
	name: string,
	url: string,
	host: string,
	-- "github", "gitlab" or "other".
	kind: string,
	owner: string,
	repo: string,
	-- https://host/owner/repo, when the url names a host.
	web_url: string?,
}

type GitDiffLine = {
	-- "add", "del" or "ctx".
	kind: string,
	-- Line numbers in the old and new file; `old` is nil on "add" lines,
	-- `new` on "del" ones.
	old: number?,
	new: number?,
	-- Cut to 300 characters.
	text: string,
}

type GitDiff = {
	path: string,
	binary: boolean,
	-- Lines in all hunks; at most 1200, then `truncated`.
	lines: number,
	truncated: boolean,
	hunks: { { header: string, old_start: number, new_start: number, lines: { GitDiffLine } } },
}

type Git = {
	available: boolean,
	reason: string?,
	repository: boolean,
	-- "HEAD" when detached.
	branch: string,
	-- Files with changes, staged, unstaged or untracked.
	changed: number,
	ahead: number,
	behind: number,
	root: string,
	detached: boolean,
	-- Nil before the first commit.
	head: { sha: string, short: string, subject: string, author: string, email: string, time: number }?,
	-- "origin/main", or nil without one.
	upstream: string?,
	-- Lines in the working tree against HEAD, tracked files only.
	added: number,
	deleted: number,
	-- At most 300, then `files_truncated`.
	files: { GitFile },
	files_truncated: boolean,
	worktrees: { GitWorktree },
	-- Recent local branches, newest first, at most 20.
	branches: { GitBranch },
	remote: GitRemote?,
	stashes: number,
	-- The file asked for with `{ diff = path }`, until `{ diff = false }`.
	diff: GitDiff?,
}

-- A forge account; `avatar_path` is a cached image file once downloaded.
type ForgePerson = {
	login: string,
	name: string?,
	bot: boolean?,
	avatar_url: string?,
	avatar_path: string?,
}

type ForgeChecks = {
	total: number,
	passed: number,
	failed: number,
	pending: number,
	skipped: number,
}

-- A pull request (GitHub) or merge request (GitLab).
type ForgePr = {
	number: number,
	title: string,
	-- "open", "closed" or "merged".
	state: string,
	draft: boolean,
	url: string,
	branch: string,
	base: string,
	author: ForgePerson?,
	-- Unix seconds.
	updated: number?,
	-- "approved", "changes_requested", "review_required" or "".
	review_decision: string,
	-- "mergeable", "conflicting", "unknown" or "".
	mergeable: string,
	-- The forge's own merge state, lowercase ("clean", "blocked", ...).
	merge_state: string,
	checks: ForgeChecks,
	-- The badge: "draft", "waiting", "ready", "attention", "merged",
	-- "closed" or "held".
	tone: string,
	-- GitHub only.
	additions: number?,
	deletions: number?,
	-- `current_pr` only: each check by name, at most 50; `outcome` is
	-- "passed", "failed", "pending" or "skipped".
	check_list: { { name: string, outcome: string, url: string? } }?,
}

type ForgeIssue = {
	number: number,
	title: string,
	url: string,
	author: ForgePerson?,
	updated: number?,
	labels: { { name: string, color: string? } },
	assignees: { string },
}

-- `app.github` and `app.gitlab`. When `available` is false only `reason`
-- is set.
type Forge = {
	available: boolean,
	reason: string?,
	repo: { name_with_owner: string, url: string, default_branch: string? },
	viewer: ForgePerson?,
	-- The current branch's, or nil.
	current_pr: ForgePr?,
	-- Open ones, most recently updated first, at most 30.
	prs: { ForgePr },
	issues: { ForgeIssue },
}

-- A coding-agent CLI running on this machine.
type Agent = {
	pid: number,
	ppid: number,
	-- "claude", "codex", "gemini", "cursor", "opencode", "aider", "amp",
	-- "copilot", "droid", "kiro", "qwen", "goose", "crush", ...
	name: string,
	-- Its command line, cut to 200 characters.
	command: string,
	cwd: string?,
	tty: string?,
	-- Unix seconds.
	started: number,
	elapsed_seconds: number,
	-- Percent of one core.
	cpu: number,
	-- Always "running": what it is doing is not visible from outside.
	status: string,
}

type Agents = {
	available: boolean,
	reason: string?,
	agents: { Agent },
	count: number,
}

-- One entry of a folder, from the `files` provider.
type FileEntry = {
	name: string,
	dir: boolean,
	-- Ignored by git: editors dim these.
	ignored: boolean,
}

type Files = {
	-- False outside a git repository unless MOONFRAME_FILES is set; then
	-- only `reason` comes with it.
	available: boolean,
	reason: string?,
	-- The root folder's name, and its absolute path.
	root: string,
	path: string,
	-- Where a terminal starts: the folder `moonframe` was started from,
	-- else `path`.
	cwd: string?,
	-- Entries of the root (key "") and of each open folder (its path from
	-- the root), folders first, each group sorted without regard to case.
	dirs: { [string]: { FileEntry } },
}

-- What this machine has for keeping terminals (the `herdr` provider).
-- The `docs` provider: the folder whose text files it reads and saves for
-- the mod that asks (`{ read = path }`, `{ write = path, text, version }`).
type Docs = {
	available: boolean,
	reason: string?,
	root: string?,
}

type Herdr = {
	installed: boolean,
	version: string?,
	path: string?,
	tmux: boolean,
	-- herdr-core's installer is running.
	installing: boolean,
	-- Why the last install failed.
	error: string?,
	-- Installed during this run.
	fresh: boolean,
}

-- `app.acp`, from providers/acp: coding agents run for mods over the
-- Agent Client Protocol.
type AcpAgent = {
	-- "claude", "codex", "copilot", "gemini" or "opencode".
	name: string,
	title: string,
	-- Its ACP command is installed.
	installed: boolean,
	-- Only `npx` can start it, downloading its adapter first.
	fetched: boolean,
}

type AcpTool = {
	id: string,
	title: string,
	-- "read", "edit", "delete", "move", "search", "execute", "think", "fetch", "other"...
	kind: string,
	-- "pending", "in_progress", "completed" or "failed".
	status: string,
}

type AcpRun = {
	id: string,
	-- The mod that started it: the only one it obeys.
	owner: string,
	agent: string,
	agent_title: string,
	-- The user's words.
	text: string,
	-- The run it follows up (`thread` in its start): the first of a
	-- conversation.
	thread: string?,
	-- "starting", "running", "waiting" (a question for the user), "done",
	-- "failed", "cancelled" or "stopped" (the app quit while it ran).
	status: string,
	-- Unix seconds.
	started: number,
	finished: number?,
	error: string?,
	stop_reason: string?,
	-- The agent's latest message: its answer once it is done.
	message: string?,
	-- The latest tool calls (at most 12), newest last, of `tool_count`.
	tools: { AcpTool },
	tool_count: number,
	plan: { { content: string, status: string } },
	-- A permission question waiting on the user; answer with
	-- `{ answer = id, option = option.id }` (or `option = false`).
	question: { title: string, options: { { id: string, name: string, kind: string } } }?,
}

type Acp = {
	available: boolean,
	agents: { AcpAgent },
	-- The agent a start without `agent` gets.
	agent: string,
	-- The folder agents work in.
	workdir: string,
	-- The app folder they change, from `workdir` (`app`, `examples/ledger`),
	-- when it is inside it.
	app: string?,
	-- This app's process: an agent screenshots the window with it.
	pid: number,
	running: number,
	-- Newest first, at most 30.
	runs: { AcpRun },
}

-- A native provider's answer to an action a mod asked for, arriving in that
-- mod's `update`: `reply` is the `id` the mod sent, else the action's name.
type Reply = {
	reply: any,
	action: string,
	ok: boolean,
	-- What happened, when it worked; why not, when it did not.
	message: string?,
	error: string?,
}

-- `ui.mount { "explorer", id = "files-2", props = { ... } }`.
type MountProps = {
	mod: string?,
	id: string?,
	props: any,
	[number]: string,
}

-- What every `view` gets as `app`.
type App = {
	dark: boolean,
	appearance: Appearance,
	window: WindowState,
	mods: { string },
	regions: { [string]: { string } },
	-- Nil until the first sample arrives (from the `system` provider).
	system: System?,
	-- Nil until each provider first publishes.
	git: Git?,
	github: Forge?,
	gitlab: Forge?,
	agents: Agents?,
	files: Files?,
	docs: Docs?,
	herdr: Herdr?,
	acp: Acp?,
	-- What tab bars can open: each component and mod with a `tab` entry.
	kinds: { TabKind },
	-- The stylesheet's `looks`: which look each kind of component takes.
	looks: { [string]: string },
	-- What each provider in app/providers/ publishes, as `app.<name>`.
	[string]: any,
}

-- What `{ to = "app:notify", msg = ... }` posts. A click on it brings the
-- window forward and sends the posting mod, if it subscribes to
-- `app/notification`, `{ topic = "app/notification", value = NotificationClick }`.
type NotifyMsg = {
	-- At most 256 bytes.
	title: string,
	-- At most 2048 bytes.
	body: string?,
	-- 1 to 128 of a-z A-Z 0-9 - _ . : /, handed back on a click.
	id: string?,
}

type NotificationClick = {
	id: string?,
	clicked: boolean,
}

-- A message `update` sends on. `to` is a mod, `"parent"`, `"shell"`,
-- `"provider:<name>"`, `"topic:<name>"`, or the app itself:
-- `"app:command"` (`msg` a `Command`), `"app:appearance"` (`msg` an
-- `AppearanceMode`), `"app:theme"` (`msg` `{ light: string?, dark: string? }`),
-- `"app:sound"` (`msg` `"default"` or a sound file's path) and
-- `"app:notify"` (`msg` a `NotifyMsg`).
type Send = {
	to: string,
	msg: Data,
}

-- `titlebar = "hidden"` puts the layout under the title bar (macOS keeps its
-- traffic lights, at `traffic_lights` from the top left) or drops the system
-- frame (Linux, Windows). Give the layout a `drag = true` row to move it.
type Window = {
	titlebar: Titlebar?,
	traffic_lights: { x: number?, y: number? }?,
	-- The size the window opens at, before the user resizes it.
	size: { width: number?, height: number? }?,
}

-- A menu item in the shell's `menus`: it sends `msg` (to the shell, or to
-- `to`) or runs `command`; `shortcut` is a key as `keys` take them, such as
-- "secondary-t". "-" between items draws a line.
type MenuItem = {
	label: string,
	msg: Data?,
	to: string?,
	command: Command?,
	shortcut: string?,
}

-- `items` holds `MenuItem`s and "-"; typed loosely so items with `msg` and
-- with `command` mix in one list. The host checks each.
type AppMenu = {
	title: string,
	items: { any },
}

-- What the About panel says besides the app's name and icon.
type About = {
	name: string?,
	version: string?,
	description: string?,
	website: string?,
	copyright: string?,
}

-- What a component or mod offers tab bars (`tabbed`): its + menu lists it,
-- and a new tab of it starts with `props`. `report` names the prop a tab
-- passes as a message, whose `url` and `title` the tab keeps (a browser's
-- `navigate`).
type TabEntry = {
	title: string?,
	icon: IconName?,
	props: any,
	report: string?,
}

-- One of `app.kinds`: what a tab bar can open.
type TabKind = {
	-- "component:terminal" or "mod:explorer".
	kind: string,
	title: string,
	icon: string?,
	props: any,
	report: string?,
}

type Mod<State> = {
	-- `props` is what an instance was mounted with (`ui.mount { "name",
	-- id = ..., props = ... }`), nil for a mod in its own place.
	init: ((previous: State?, props: any?) -> State)?,
	-- Returns nothing to keep the state it changed in place, or the new
	-- state and optionally a list of `Send`s. Its results are left untyped
	-- so it may return nothing on some paths; the host checks them. A
	-- subscribed topic arrives as `{ topic = "<name>", value = ... }`.
	update: ((state: State, msg: any, props: any?) -> ...any)?,
	view: (state: State, app: App, props: any?) -> Child,
	-- Topics it hears in `update`: names, or prefixes ending in `*`.
	-- `provider/<name>` carries a provider's data as it changes.
	subscribe: { string }?,
	-- Offered as a kind of tab: each tab mounts an instance of it.
	tab: TabEntry?,
	region: string?,
	order: number?,
	title: string?,
	-- Rules for what this mod draws, by selector.
	styles: { [string]: Style }?,
	-- Only in main.luau: the window's frame, read when the app starts.
	window: Window?,
	-- Only in main.luau: menus after the app menu (a "View" one merges into
	-- the host's), and what About says.
	menus: { AppMenu }?,
	about: About?,
}

declare function mod<State>(definition: Mod<State>): Mod<State>

-- The fixed-width font terminals and `font = "mono"` text use.
type MonoFont = {
	family: string?,
	size: number?,
	-- 100 (thin) to 900 (black); 400 is regular.
	weight: number?,
	-- Terminal line height as a multiple of the size.
	line_height: number?,
	-- Fonts to look in for characters the family lacks, such as Nerd Font
	-- icons; by default { "Symbols Nerd Font Mono" }, and {} for none.
	fallbacks: { string }?,
	-- false keeps sequences such as `->` from joining.
	ligatures: boolean?,
}

type Stylesheet = {
	font_size: number?,
	font_family: string?,
	mono: MonoFont?,
	radius: number?,
	-- The look each kind of component takes unless its code picks one:
	-- `{ tabs = "cards" }`. Views read it as `app.looks`.
	looks: { [string]: string }?,
	-- Each one hex color, or `{ light = ..., dark = ... }`. Typed `any`:
	-- Luau cannot check a map of unions against a table literal, so the host
	-- checks each entry when it loads the stylesheet.
	colors: { [string]: any }?,
	-- Named durations: milliseconds, or `"220ms"`; `any` as for colors.
	speeds: { [string]: any }?,
	-- Divides every duration: 2 is twice as fast, 0 turns animations off;
	-- otherwise 0.01 to 10.
	animation_speed: number?,
	-- Styles by selector: `panel`, `.class`, `item:selected`.
	rules: { [string]: Style }?,
}

declare function style(sheet: Stylesheet): Stylesheet

-- A component in components/<name>.luau: a ready-made piece of view that
-- mods `use`, listed in the Components menu with a preview of `example`.
type Component<Props> = {
	title: string?,
	description: string?,
	-- The Components menu's submenu.
	category: string?,
	-- When to reach for it, and what to use instead; the catalogue's lead.
	when: string?,
	-- Each prop and what it does, for the catalogue.
	props: { [string]: string }?,
	-- Default rules for the classes it draws, under every mod's and the
	-- stylesheet's, so `style.luau` restyles it.
	styles: { [string]: Style }?,
	-- Shown beside its title in the browser.
	icon: IconName?,
	-- Luau that uses it; by default made from `example`.
	usage: string?,
	-- Props for the preview.
	example: any,
	-- With `update`, it keeps state: each `use` call with an `id` mounts an
	-- instance, and `build` gets `(props, state, app)`.
	build: (props: Props, state: any?, app: App?) -> Child,
	init: ((previous: any?, props: Props) -> any)?,
	-- Sends `to = "parent"` reach the module that drew it.
	update: ((state: any, msg: any, props: Props) -> ...any)?,
	subscribe: { string }?,
	tab: TabEntry?,
}

declare function component<Props>(definition: Component<Props>): Component<Props>

-- The component `name`'s `build`: `local stat = use "stat"`, then
-- `stat { value = "12%" }` wherever an element goes.
declare function use(name: string): (props: any) -> Element

-- Prints to the terminal that runs the app.
declare function log(...: any): ()
