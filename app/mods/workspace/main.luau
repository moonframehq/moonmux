-- The workspace: spaces, each with tabs, each tab a tree of split panes.
-- It draws the chosen space's tab strip (the window's title bar) over its
-- panes, and keeps the whole model in its state, which Moonframe saves.
--
-- A space holds work in one folder. A tab's `layout` is a tree: a leaf is
-- `{ pane = "p3" }`, a split is `{ id = "x4", dir = "row" | "column",
-- children = { ... } }`. Each pane is an instance of the `pane` mod with a
-- shell of its own in herdr-core or tmux; closing a pane ends its shell
-- (`provider:core`'s `close_panes`), closing the window does not.
--
-- Which backend a pane runs on is chosen when it opens and kept for its
-- life, across launches, in `backends`: the choice in Settings > Terminals
-- (`topic:moonmux/backend`), Auto meaning what `app.core.backends.auto` says
-- (herdr-core when installed, else tmux). Changing the choice applies to
-- panes opened after it; open panes keep theirs, as Moonframe keeps a
-- terminal on the backend it opened on. Once `app.core` lists a pane's
-- shell, `backends` follows where it really runs (Moonframe falls back to
-- another backend when the one asked for is missing), so the next launch
-- reattaches it there rather than starting a second shell elsewhere.
--
-- After each change it publishes `topic:moonmux/workspace` (the spaces, their
-- panes, which pane shows where) for the sidebar, and the focused pane with
-- its space's folder for the Files and Changes panels.
--
-- A tab started for an agent (`{ start_agent = "claude" }`, from mods/new-menu)
-- keeps it in `starts` until `app.core` shows the new pane's shell, then
-- asks `provider:core` to type it there (`{ start = pane, program }`) and
-- forgets it: once per pane, and the provider labels the terminal so it is
-- never typed into twice.

local icon_button = use "icon-button"
local status_dot = use "status-dot"
local program_icon = use "program-icon"

type Node = { pane: string?, id: string?, dir: string?, children: { any }? }
type Tab = { id: string, name: string, layout: Node, active: string }
type Space = { id: string, name: string, cwd: string, tabs: { Tab }, tab: string }
type Dialog = { kind: string, id: string, name: string, cwd: string }
type State = {
	spaces: { Space },
	space: string,
	next: number,
	sidebar: boolean,
	dialog: Dialog?,
	-- What Settings > Look (mods/settings) chose: { tabs, headers }.
	layout: any,
	-- The right-hand panel the shell shows ("files", "changes"), if any.
	panel: string?,
	-- The agent each new pane is to start, by pane, until it is sent.
	starts: { [string]: string }?,
	-- The backend each pane runs on, by pane: `herdr-core`, `tmux`, or
	-- `auto` until `app.core` shows it. A pane missing here predates the
	-- choice and opened on herdr-core.
	backends: { [string]: string }?,
	-- Settings > Terminals' choice: `auto`, `herdr-core` or `tmux`.
	backend: string?,
	-- From `app.core.backends`: Auto's backend, and the installed ones.
	auto: string?,
	installed: { string }?,
}

-- How loud a status is, for a tab or space showing its loudest agent.
local LOUDNESS = { blocked = 4, working = 3, done = 2, idle = 1 }

local function fresh_id(state: State, prefix: string): string
	state.next += 1
	return `{prefix}{state.next}`
end

-- The backend a pane opened now runs on: the choice when it is installed,
-- else Auto's; `auto` (Moonframe's own pick, the same) before `app.core`
-- has said.
local function chosen_backend(state: State): string
	local choice = state.backend or "auto"
	if choice ~= "auto" and table.find(state.installed or {}, choice) then
		return choice
	end
	return if state.auto and state.auto ~= "" then state.auto else "auto"
end

-- A new pane's id, with its backend kept.
local function new_pane(state: State): string
	local pane = fresh_id(state, "p")
	local backends = state.backends or {}
	backends[pane] = chosen_backend(state)
	state.backends = backends
	return pane
end

local function new_tab(state: State, cwd: string): Tab
	local pane = new_pane(state)
	return { id = fresh_id(state, "t"), name = "", layout = { pane = pane }, active = pane }
end

local function new_space(state: State, name: string, cwd: string): Space
	local tab = new_tab(state, cwd)
	return { id = fresh_id(state, "s"), name = name, cwd = cwd, tabs = { tab }, tab = tab.id }
end

local function find_space(state: State, id: string?): (Space?, number)
	for index, space in state.spaces do
		if space.id == id then
			return space, index
		end
	end
	return nil, 0
end

local function current_space(state: State): Space?
	return (find_space(state, state.space))
end

local function find_tab(space: Space, id: string?): (Tab?, number)
	for index, tab in space.tabs do
		if tab.id == id then
			return tab, index
		end
	end
	return nil, 0
end

local function current_tab(state: State): Tab?
	local space = current_space(state)
	return if space then (find_tab(space, space.tab)) else nil
end

-- Every pane under `node`, in order.
local function panes_of(node: Node, into: { string }?): { string }
	local list = into or {}
	if node.pane then
		table.insert(list, node.pane)
	else
		local children: { any } = node.children or {}
		for _, child in children do
			panes_of(child, list)
		end
	end
	return list
end

local function has_pane(node: Node, pane: string): boolean
	return table.find(panes_of(node), pane) ~= nil
end

-- Splits the leaf `pane` along `dir`, putting `new` after it: into its
-- parent when that already runs along `dir`, else in a split of its own.
local function split(state: State, node: Node, pane: string, dir: string, new: string): Node
	if node.pane then
		if node.pane == pane then
			return { id = fresh_id(state, "x"), dir = dir, children = { node, { pane = new } } }
		end
		return node
	end
	local children: { any } = node.children or {}
	for index, child in children do
		if child.pane == pane and node.dir == dir then
			table.insert(children, index + 1, { pane = new })
			return node
		end
		children[index] = split(state, child, pane, dir, new)
	end
	return node
end

-- `node` without the leaf `pane`; a split left with one child becomes it.
local function without(node: Node, pane: string): Node?
	if node.pane then
		return if node.pane == pane then nil else node
	end
	local kept = {}
	local children: { any } = node.children or {}
	for _, child in children do
		local left = without(child, pane)
		if left then
			table.insert(kept, left)
		end
	end
	if #kept == 0 then
		return nil
	elseif #kept == 1 then
		return kept[1]
	end
	node.children = kept
	return node
end

-- The folder's last part: `~/code/herdr` is `herdr`.
local function base(path: string?): string
	if not path or path == "" or path == "~" or path == "/" then
		return path or "~"
	end
	return string.match(path, "([^/]+)/*$") or path
end

-- The pending starts whose pane's shell `core` (app.core) now shows, as
-- sends to `provider:core`; each is forgotten as it is sent, and those of
-- panes since closed are dropped.
local function start_sends(state: State, core: any): { any }?
	local starts = state.starts
	if not starts or next(starts) == nil then
		return nil
	end
	local live = {}
	for _, terminal in (core and core.terminals) or {} do
		if terminal.ours and terminal.pane ~= "" then
			live[terminal.pane] = true
		end
	end
	local open = {}
	for _, space in state.spaces do
		for _, tab in space.tabs do
			panes_of(tab.layout, open)
		end
	end
	local sends = {}
	for pane, program in starts do
		if live[pane] then
			table.insert(sends, { to = "provider:core", msg = { start = pane, program = program } })
			starts[pane] = nil
		elseif not table.find(open, pane) then
			starts[pane] = nil
		end
	end
	return sends
end

local function close_sends(panes: { string }): { any }
	if #panes == 0 then
		return {}
	end
	return { { to = "provider:core", msg = { close_panes = table.concat(panes, ",") } } }
end

-- What the sidebar needs, published after each change.
local function summary(state: State)
	local spaces = {}
	local visible = {}
	local where = {}
	for _, space in state.spaces do
		local panes = {}
		for _, tab in space.tabs do
			for _, pane in panes_of(tab.layout) do
				table.insert(panes, pane)
				where[pane] = space.id
				if space.id == state.space and tab.id == space.tab then
					table.insert(visible, pane)
				end
			end
		end
		table.insert(spaces, { id = space.id, name = space.name, cwd = space.cwd, tabs = #space.tabs, panes = panes })
	end
	-- The focused pane: the current tab's active one, and its space's
	-- folder for when herdr-core has not said where its shell is.
	local space = current_space(state)
	local tab = current_tab(state)
	local focus = if space and tab then { pane = tab.active, cwd = space.cwd } else nil
	return {
		to = "topic:moonmux/workspace",
		msg = { space = state.space, spaces = spaces, visible = visible, where = where, focus = focus },
	}
end

-- Follows `core` (app.core) for where our panes' shells run: a pane whose
-- kept backend has no shell of it, but another backend has one, moves to
-- that one. Says whether anything changed.
local function follow_backends(state: State, core: any): boolean
	local moved = false
	local found: { [string]: { string } } = {}
	for _, terminal in (core and core.terminals) or {} do
		if terminal.ours and terminal.pane ~= "" and terminal.backend then
			found[terminal.pane] = found[terminal.pane] or {}
			table.insert(found[terminal.pane], terminal.backend)
		end
	end
	local backends = state.backends or {}
	for pane, runs_on in found do
		if not table.find(runs_on, backends[pane] or "herdr-core") then
			backends[pane] = runs_on[1]
			moved = true
		end
	end
	state.backends = backends
	local list = (core and core.backends and core.backends.list) or {}
	local installed = {}
	for _, backend in list do
		if backend.installed then
			table.insert(installed, backend.id)
		end
	end
	local auto = (core and core.backends and core.backends.auto) or ""
	if auto ~= state.auto or table.concat(installed, ",") ~= table.concat(state.installed or {}, ",") then
		state.auto, state.installed = auto, installed
		moved = true
	end
	return moved
end

-- Puts `sends` after the summary: every change ends here. Forgets the
-- backends of panes no longer open.
local function changed(state: State, sends: { any }?)
	local open = {}
	for _, space in state.spaces do
		for _, tab in space.tabs do
			panes_of(tab.layout, open)
		end
	end
	local backends: { [string]: string } = state.backends or {}
	for pane in backends do
		if not table.find(open, pane) then
			backends[pane] = nil
		end
	end
	state.backends = backends
	local all = { summary(state) }
	local extra: { any } = sends or {}
	for _, send in extra do
		table.insert(all, send)
	end
	return state, all
end

local function select_space(state: State, step: number)
	local _, index = find_space(state, state.space)
	local count = #state.spaces
	if count > 0 then
		state.space = state.spaces[(index - 1 + step) % count + 1].id
	end
end

local function select_tab(space: Space, step: number)
	local _, index = find_tab(space, space.tab)
	local count = #space.tabs
	if count > 0 then
		space.tab = space.tabs[(index - 1 + step) % count + 1].id
	end
end

-- Closes `tab` of `space`, and the space too when it was its last.
local function close_tab(state: State, space: Space, tab: Tab)
	local closing = panes_of(tab.layout)
	local _, index = find_tab(space, tab.id)
	table.remove(space.tabs, index)
	if #space.tabs == 0 then
		local _, at = find_space(state, space.id)
		table.remove(state.spaces, at)
		if #state.spaces == 0 then
			table.insert(state.spaces, new_space(state, "", "~"))
		end
		local next = state.spaces[math.min(at, #state.spaces)]
		if state.space == space.id then
			state.space = next.id
		end
	elseif space.tab == tab.id then
		space.tab = space.tabs[math.min(index, #space.tabs)].id
	end
	return changed(state, close_sends(closing))
end

local function close_pane(state: State, space: Space, tab: Tab, pane: string)
	local left = without(tab.layout, pane)
	if not left then
		return close_tab(state, space, tab)
	end
	tab.layout = left
	if tab.active == pane then
		tab.active = panes_of(left)[1]
	end
	return changed(state, close_sends({ pane }))
end

-- A title bar button showing or hiding a right-hand panel in the shell.
local function panel_button(state: State, panel: string, label: string, icon: string)
	return ui.row {
		class = "ws-panel-button",
		align = "center",
		shrink = false,
		selected = state.panel == panel,
		msg = { panel = panel },
		to = "shell",
		tooltip = if state.panel == panel then `Hide {label}` else `Show {label}`,
		ui.icon { name = icon :: any, size = "small" },
		ui.text { label, size = "small" },
	}
end

return mod {
	title = "Workspace",
	subscribe = { "moonmux/layout", "moonmux/backend", "provider/core" },

	styles = {
		[".ws-strip"] = { padding = { 0, 10, 0, 12 }, gap = 4 },
		[".ws-tab"] = { padding = { 5, 8, 5, 12 }, radius = 8, gap = 6, color = "muted-foreground" },
		[".ws-tab:hover"] = { background = "list-hover" },
		[".ws-tab:selected"] = { background = "card", color = "foreground", shadow = "0 1px 2px #00000018" },
		[".ws-tab-underline"] = { padding = { 9, 8, 7, 10 }, gap = 6, color = "muted-foreground", border = "bottom", border_color = "#00000000" },
		[".ws-tab-underline:hover"] = { color = "foreground" },
		[".ws-tab-underline:selected"] = { color = "foreground", border_color = "accent" },
		[".ws-tab-cards"] = { padding = { 6, 8, 6, 12 }, gap = 6, radius = { 8, 8, 0, 0 }, color = "muted-foreground", border = true, border_color = "border", background = "list-hover" },
		[".ws-tab-cards:hover"] = { color = "foreground" },
		[".ws-tab-cards:selected"] = { background = "card", color = "foreground" },
		[".ws-tool"] = { width = 28, height = 28, radius = 7, color = "muted-foreground" },
		[".ws-tool:hover"] = { background = "list-hover", color = "foreground" },
		[".ws-panel-button"] = { padding = { 4, 8 }, radius = 7, gap = 5, color = "muted-foreground" },
		[".ws-panel-button:hover"] = { background = "list-hover", color = "foreground" },
		[".ws-panel-button:selected"] = { background = "list-hover", color = "foreground" },
		[".ws-tools-gap"] = { width = 1, height = 18, background = "border", margin = { 0, 4 } },
		[".ws-card"] = {
			background = "card",
			radius = 12,
			border = true,
			border_color = "border",
			shadow = "0 1px 2px #0000000d, 0 6px 20px -6px #00000026",
			clip = true,
		},
		[".ws-pane-head"] = { padding = { 3, 6, 3, 10 }, gap = 6, border = "bottom", border_color = "border" },
		[".ws-pane-head:active"] = { background = "accent/8" },
		[".ws-pane-name"] = { size = "small", weight = "medium", color = "muted-foreground" },
		[".ws-pane-name:active"] = { color = "foreground" },
		[".ws-pane-title"] = { size = "small", color = "muted-foreground" },
		[".ws-pane-icon"] = { color = "muted-foreground" },
		[".ws-pane-backend"] = { size = "small", color = "muted-foreground", padding = { 0, 6 }, radius = 5, background = "list-hover" },
		[".ws-field"] = { gap = 4 },
		[".ws-dialog"] = { background = "card", radius = 14, padding = 18, gap = 14, shadow = "large" },
	},

	init = function(previous: State?, props): State
		if previous and previous.spaces and #previous.spaces > 0 then
			previous.dialog = nil
			previous.sidebar = previous.sidebar ~= false
			previous.starts = previous.starts or {}
			previous.backends = previous.backends or {}
			return previous, { summary(previous) }
		end
		local state: State = { spaces = {}, space = "", next = 0, sidebar = true, dialog = nil, layout = nil, starts = {}, backends = {} }
		local first = new_space(state, "", "~")
		state.spaces = { first }
		state.space = first.id
		return state, { summary(state) }
	end,

	update = function(state: State, msg)
		local space = current_space(state)
		local tab = current_tab(state)
		if type(msg) == "string" then
			if msg == "new-tab" and space then
				local made = new_tab(state, space.cwd)
				table.insert(space.tabs, made)
				space.tab = made.id
			elseif (msg == "split-right" or msg == "split-down") and tab then
				local pane = new_pane(state)
				tab.layout = split(state, tab.layout, tab.active, if msg == "split-right" then "row" else "column", pane)
				tab.active = pane
			elseif msg == "close-pane" and space and tab then
				return close_pane(state, space, tab, tab.active)
			elseif msg == "close-tab" and space and tab then
				return close_tab(state, space, tab)
			elseif msg == "next-tab" and space then
				select_tab(space, 1)
			elseif msg == "prev-tab" and space then
				select_tab(space, -1)
			elseif msg == "next-space" then
				select_space(state, 1)
			elseif msg == "prev-space" then
				select_space(state, -1)
			elseif msg == "new-space" then
				state.dialog = { kind = "space", id = "", name = "", cwd = if space then space.cwd else "~" }
			elseif msg == "dialog-close" then
				state.dialog = nil
			elseif msg == "dialog-submit" and state.dialog then
				local dialog = state.dialog
				local name = string.match(dialog.name, "^%s*(.-)%s*$") or ""
				local cwd = string.match(dialog.cwd, "^%s*(.-)%s*$") or ""
				if dialog.kind == "space" then
					local made = new_space(state, name, if cwd == "" then "~" else cwd)
					table.insert(state.spaces, made)
					state.space = made.id
				elseif dialog.kind == "rename-space" then
					local target = find_space(state, dialog.id)
					if target then
						target.name = name
					end
				elseif dialog.kind == "rename-tab" and space then
					local target = find_tab(space, dialog.id)
					if target then
						target.name = name
					end
				end
				state.dialog = nil
			else
				return
			end
			return changed(state)
		end
		if type(msg) ~= "table" then
			return
		end
		if msg.topic == "moonmux/layout" then
			state.layout = msg.value
			return state
		elseif msg.topic == "moonmux/backend" then
			state.backend = (msg.value and msg.value.choice) or "auto"
			return state
		elseif msg.topic == "provider/core" then
			local moved = follow_backends(state, msg.value)
			local sends = start_sends(state, msg.value)
			if not sends and not moved then
				return
			end
			return state, sends or {}
		elseif msg.start_agent and space then
			-- A tab for the agent, in the current space; its pane starts it
			-- once its shell is up (see start_sends).
			local made = new_tab(state, space.cwd)
			table.insert(space.tabs, made)
			space.tab = made.id
			local starts = state.starts or {}
			starts[made.active] = msg.start_agent
			state.starts = starts
		elseif msg.msg == "dialog-name" and state.dialog then
			state.dialog.name = msg.value
			return state
		elseif msg.msg == "dialog-cwd" and state.dialog then
			state.dialog.cwd = msg.value
			return state
		elseif msg.sidebar ~= nil then
			state.sidebar = msg.sidebar
			return state
		elseif msg.panel ~= nil then
			state.panel = if msg.panel == false then nil else msg.panel
			return state
		elseif msg.tab_index and space then
			local index = if msg.tab_index < 0 then #space.tabs else msg.tab_index
			if space.tabs[index] then
				space.tab = space.tabs[index].id
			end
		elseif msg.select_tab and space then
			space.tab = msg.select_tab
		elseif msg.close_tab and space then
			local target = find_tab(space, msg.close_tab)
			if target then
				return close_tab(state, space, target)
			end
		elseif msg.select_space then
			state.space = msg.select_space
		elseif msg.close_space then
			local target, index = find_space(state, msg.close_space)
			if target then
				local closing = {}
				for _, each in target.tabs do
					panes_of(each.layout, closing)
				end
				table.remove(state.spaces, index)
				if #state.spaces == 0 then
					table.insert(state.spaces, new_space(state, "", "~"))
				end
				if state.space == target.id then
					state.space = state.spaces[math.min(index, #state.spaces)].id
				end
				return changed(state, close_sends(closing))
			end
		elseif msg.rename_space then
			local target = find_space(state, msg.rename_space)
			if target then
				state.dialog = { kind = "rename-space", id = target.id, name = target.name, cwd = target.cwd }
			end
		elseif msg.rename_tab and space then
			local target = find_tab(space, msg.rename_tab)
			if target then
				state.dialog = { kind = "rename-tab", id = target.id, name = target.name, cwd = "" }
			end
		elseif msg.focus_pane then
			-- From the sidebar: show the space and tab the pane is in.
			for _, each in state.spaces do
				for _, candidate in each.tabs do
					if has_pane(candidate.layout, msg.focus_pane) then
						state.space = each.id
						each.tab = candidate.id
						candidate.active = msg.focus_pane
					end
				end
			end
		elseif msg.active and tab then
			tab.active = msg.active
		elseif msg.split and tab then
			local pane = new_pane(state)
			tab.layout = split(state, tab.layout, msg.split, msg.dir or "row", pane)
			tab.active = pane
		elseif msg.close and space and tab then
			return close_pane(state, space, tab, msg.close)
		else
			return
		end
		return changed(state)
	end,

	view = function(state: State, app)
		local space = current_space(state)
		local tab = current_tab(state)
		local mac = app.window.platform == "macos"
		local hidden = app.window.titlebar == "hidden"
		local layout = state.layout or {}
		local tab_class = if layout.tabs == "underline" then "ws-tab-underline"
			elseif layout.tabs == "cards" then "ws-tab-cards"
			else "ws-tab"
		local headers = layout.headers or "always"
		local starts = state.starts or {}
		local backends = state.backends or {}

		-- What `app.core` says runs in each of our panes, and which panes'
		-- shells exited.
		local ended = {}
		for _, pane in (app.core and app.core.ended) or {} do
			ended[pane] = true
		end
		local by_pane = {}
		for _, terminal in (app.core and app.core.terminals) or {} do
			if terminal.ours and terminal.pane ~= "" then
				by_pane[terminal.pane] = terminal
			end
		end

		-- The key of the program a pane shows (its icon), and its name:
		-- the agent it is about to start, else what `app.core` says runs.
		local function program_of(pane: string): (string, string)
			local terminal = by_pane[pane]
			local starting = starts[pane]
			if starting and not (terminal and terminal.is_agent) then
				for _, program in (app.core and app.core.programs) or {} do
					if program.key == starting then
						return starting, program.name
					end
				end
			end
			if not terminal then
				return "shell", ""
			end
			return terminal.program_key or "", if terminal.program and terminal.program ~= "" then terminal.program else terminal.agent
		end

		local function loudest(node: Node): string?
			local best, level = nil, 0
			for _, pane in panes_of(node) do
				local terminal = by_pane[pane]
				local status = terminal and terminal.status or ""
				if (LOUDNESS[status] or 0) > level then
					best, level = status, LOUDNESS[status]
				end
			end
			return best
		end

		local function tab_label(each: Tab): string
			if each.name ~= "" then
				return each.name
			end
			local pane = if by_pane[each.active] then each.active else panes_of(each.layout)[1]
			local terminal = by_pane[pane]
			local key, name = program_of(pane)
			if key ~= "shell" and name ~= "" and (starts[pane] or (terminal and terminal.is_agent)) then
				return name
			end
			return base(if terminal then terminal.cwd else (space and space.cwd))
		end

		-- The tab strip, which is the window's title bar.
		local tabs = {}
		local space_tabs: { Tab } = if space then space.tabs else {}
		for index, each in space_tabs do
			local status = loudest(each.layout)
			local shown = if by_pane[each.active] then each.active else panes_of(each.layout)[1]
			local terminal = by_pane[shown]
			table.insert(tabs, ui.row {
				key = each.id,
				class = tab_class,
				align = "center",
				selected = space and each.id == space.tab,
				msg = { select_tab = each.id },
				context = { rename_tab = each.id },
				tooltip = if index <= 8 then `⌘{index}` else nil,
				status and status_dot { status = status, size = "small" },
				program_icon { program = (program_of(shown)), agent = terminal and terminal.is_agent, dark = app.dark },
				ui.row { max_width = 180, ui.text { tab_label(each), truncate = true } },
				icon_button { icon = "close", size = "small", tooltip = "Close tab", msg = { close_tab = each.id } },
			})
		end
		local strip = ui.row {
			class = "ws-strip",
			drag = true,
			height = 46,
			shrink = false,
			align = "center",
			-- Room for the traffic lights while the sidebar does not hold them.
			hidden and mac and not state.sidebar and not app.window.fullscreen and ui.row { width = 72, shrink = false },
			not state.sidebar and icon_button { icon = "panel-left", tooltip = "Show sidebar (⌘B)", msg = "toggle-left", to = "shell" },
			ui.row { gap = "small", align = "center", table.unpack(tabs) },
			-- A shell, an agent or a space (mods/new-menu).
			ui.mount "new-menu",
			ui.spacer(),
			-- The right-hand panels, which the shell shows one at a time.
			panel_button(state, "files", "Files", "file"),
			panel_button(state, "changes", "Changes", "git-compare"),
			tab and icon_button { icon = "columns-2", tooltip = "Split right (⌘D)", msg = { split = tab.active, dir = "row" } },
			tab and icon_button { icon = "rows-2", tooltip = "Split down (⇧⌘D)", msg = { split = tab.active, dir = "column" } },
			ui.row { class = "ws-tools-gap" },
			-- The layout choices, in Settings > Look (mods/settings).
			ui.row { class = "ws-tool", align = "center", justify = "center", msg = { open = "look" }, to = "settings", tooltip = "Layout", ui.icon { "layout-dashboard" } },
			-- Send feedback (the built-in mods/feedback): ask an agent to change this app.
			ui.mount "feedback",
			ui.row { class = "ws-tool", align = "center", justify = "center", command = "show-themes", tooltip = "Themes", ui.icon { "palette" } },
			-- Linux and Windows draw no window buttons without a title bar.
			hidden and not mac and ui.row {
				gap = "none",
				ui.button { "–", variant = "ghost", small = true, command = "minimize" },
				ui.button { if app.window.maximized then "❐" else "□", variant = "ghost", small = true, command = "maximize" },
				ui.button { "×", variant = "ghost", small = true, command = "close-window" },
			},
		}

		local function pane_view(pane: string, many: boolean)
			local active = tab ~= nil and tab.active == pane
			local terminal = by_pane[pane]
			local show_head = headers == "always" or (headers == "split" and many)
			local key, name = program_of(pane)
			local runs_on = if terminal and terminal.backend then terminal.backend else (backends[pane] or "herdr-core")
			-- As herdr-app: the program's icon and name, then the live folder.
			local folder = if terminal then terminal.cwd
				elseif ended[pane] then "The shell exited: click the pane and press Enter"
				else "Starting…"
			local head = if not show_head then nil else ui.row {
				class = "ws-pane-head",
				state = if active then "active" else "rest",
				align = "center",
				shrink = false,
				msg = { active = pane },
				tooltip = terminal and `{terminal.backend} {terminal.target}`,
				ui.row { class = "ws-pane-icon", shrink = false, program_icon { program = key, agent = terminal and terminal.is_agent, dark = app.dark, size = "small" } },
				name ~= "" and ui.row { max_width = 260, shrink = false, ui.text { name, class = "ws-pane-name", state = if active then "active" else "rest", truncate = true } },
				ui.row { grow = true, ui.text { folder, class = "ws-pane-title", truncate = true } },
				terminal and terminal.status ~= "" and status_dot { status = terminal.status, size = "small" },
				-- The backend its shell runs on.
				runs_on ~= "auto" and ui.row {
					class = "ws-pane-backend",
					shrink = false,
					tooltip = `Its shell runs on {runs_on}. Settings › Terminals picks where new panes run.`,
					ui.text { runs_on },
				},
				icon_button { icon = "columns-2", size = "small", tooltip = "Split right", msg = { split = pane, dir = "row" } },
				icon_button { icon = "rows-2", size = "small", tooltip = "Split down", msg = { split = pane, dir = "column" } },
				icon_button { icon = "close", size = "small", tooltip = "Close pane", msg = { close = pane } },
			}
			return ui.column {
				grow = true,
				gap = "none",
				head,
				-- A click in the terminal makes its pane the active one, so
				-- Files and Changes follow the terminal being typed in. The
				-- terminal takes focus on the press and lets the click
				-- through; the innermost `msg` wins, so the header's own
				-- buttons are untouched.
				ui.column {
					grow = true,
					gap = "none",
					msg = if active then nil else { active = pane },
					ui.mount { "pane", id = pane, props = { cwd = space and space.cwd or "~", backend = backends[pane] or "herdr-core" } },
				},
			}
		end

		local function draw(node: Node, many: boolean): any
			if node.pane then
				return pane_view(node.pane, many)
			end
			local panes = {}
			local children: { any } = node.children or {}
			for _, child in children do
				table.insert(panes, ui.pane { min = 120, draw(child, true) })
			end
			if node.dir == "column" then
				return ui.vsplit { id = node.id or "split", table.unpack(panes) }
			end
			return ui.hsplit { id = node.id or "split", table.unpack(panes) }
		end

		local content = if tab
			then draw(tab.layout, tab.layout.pane == nil)
			else ui.column { grow = true, align = "center", justify = "center", ui.text { "No tab", tone = "muted" } }

		-- New space and renames.
		local dialog = state.dialog
		local title = if not dialog then ""
			elseif dialog.kind == "space" then "New space"
			elseif dialog.kind == "rename-space" then "Rename space"
			else "Rename tab"
		local overlay = ui.overlay {
			open = dialog ~= nil,
			dismiss = "dialog-close",
			backdrop = true,
			width = 440,
			class = "ws-dialog",
			keys = { enter = "dialog-submit" :: any },
			content = ui.column {
				gap = "large",
				ui.text { title, size = "large", weight = "semibold" },
				dialog and dialog.kind == "space" and ui.column {
					class = "ws-field",
					ui.text { "Folder", size = "small", tone = "muted" },
					ui.input {
						id = "dialog-cwd",
						value = dialog.cwd,
						placeholder = "~/code/project",
						change = "dialog-cwd",
						submit = "dialog-submit",
						mono = true,
						focus = true,
					},
				},
				ui.column {
					class = "ws-field",
					ui.text { if dialog and dialog.kind == "space" then "Name (optional: it follows the folder)" else "Name", size = "small", tone = "muted" },
					ui.input {
						id = "dialog-name",
						value = dialog and dialog.name or "",
						placeholder = if dialog and dialog.kind == "space" then base(dialog.cwd) else "Follows what runs in it",
						change = "dialog-name",
						submit = "dialog-submit",
						focus = dialog ~= nil and dialog.kind ~= "space",
					},
				},
				ui.row {
					justify = "end",
					ui.button { "Cancel", variant = "ghost", msg = "dialog-close" },
					ui.button { if dialog and dialog.kind == "space" then "Create" else "Rename", variant = "primary", msg = "dialog-submit" },
				},
			},
		}

		return ui.column {
			grow = true,
			gap = "none",
			strip,
			ui.column { class = "ws-card", grow = true, gap = "none", content },
			overlay,
		}
	end,
}
