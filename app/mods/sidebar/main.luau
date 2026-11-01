-- The sidebar: the spaces (from the workspace's `herdr/workspace` topic),
-- then every agent herdr-core runs (`app.core`): first those in this app's
-- panes, then those elsewhere (another app, `hc run`, an agent's workers).
--
-- An agent is Working, Blocked (it needs you), Done (it finished while you
-- looked elsewhere) or Idle. A done agent you have looked at (its pane is
-- on screen, or you peeked at it since it finished) shows as Idle.
-- Clicking one of ours shows its pane; clicking any other peeks at it.
--
-- The agents list as herdr-app's: by default (Priority, the clock) under
-- status headings, most urgent first and the latest change first within
-- one; or grouped by space (the layers button flips it). Each row says
-- where it runs, the program, and how long it has been in its state. An
-- agent's helpers (terminals it started, `parent` in herdr-core) show
-- indented under it, and a parent counts as urgent as its most urgent one.

local icon_button = use "icon-button"
local status_dot = use "status-dot"
local group_header = use "group-header"

type State = {
	workspace: any,
	seen: { [string]: number },
	collapsed: { [string]: boolean? },
	blocked: string?,
	-- "priority" (status headings) or "space" (a heading per space).
	sort: string,
}

local AGENT_ICONS = {
	["claude-code"] = "agent-claude", claude = "agent-claude", codex = "agent-codex", gemini = "agent-gemini",
	cursor = "agent-cursor", ["cursor-agent"] = "agent-cursor", copilot = "agent-copilot",
	opencode = "agent-opencode", amp = "agent-amp", droid = "agent-droid", kiro = "agent-kiro",
	qwen = "agent-qwen", grok = "agent-grok",
}

local LABELS = { working = "Working", blocked = "Needs you", done = "Done", idle = "Idle" }
local LOUDNESS = { blocked = 4, working = 3, done = 2, idle = 1 }
-- The status headings, most urgent first.
local SECTIONS = { "blocked", "working", "done", "idle" }
-- The two orders, and the button that flips between them: it shows the
-- current one, as herdr-app's does.
local SORTS = {
	priority = { label = "Priority", icon = "clock", next = "space" },
	space = { label = "Grouped by space", icon = "layers", next = "priority" },
}
-- How deep helpers of helpers nest, against a loop in `parent`.
local MAX_DEPTH = 6
-- Pixels of indent per helper level.
local INDENT = 14
local ROW = 28

local function base(path: string?): string
	if not path or path == "" or path == "~" or path == "/" then
		return path or "~"
	end
	return string.match(path, "([^/]+)/*$") or path
end

-- The status to show: a done agent already looked at is idle.
local function shown_status(terminal, seen: { [string]: number }, visible: { [string]: boolean }): string
	local status = terminal.status
	if status == "" then
		return "idle"
	end
	if status == "done" and (visible[terminal.pane] or seen[terminal.address] == terminal.turn) then
		return "idle"
	end
	return status
end

-- A helper's status: it reports to its parent, whose own done is the news,
-- so a helper's done shows as idle; an error is something that needs you.
local function helper_status(terminal): string
	local status = terminal.status
	if status == "error" then
		return "blocked"
	elseif status == "done" or status == "" then
		return if terminal.is_agent then "idle" else "unknown"
	end
	return if LOUDNESS[status] then status else "unknown"
end

-- How long it has been in its state: "<1m", "4m", "2h", "3d".
local function age(ms: number): string
	local minutes = ms // 60000
	if minutes < 1 then
		return "<1m"
	elseif minutes < 60 then
		return `{minutes}m`
	end
	local hours = minutes // 60
	return if hours < 24 then `{hours}h` else `{hours // 24}d`
end

-- One cell of guide lines in front of a helper, as the tree component
-- draws them: `pipe` (│), `tee` (├), `elbow` (└) or `blank`.
local function guide(kind: string)
	if kind == "blank" then
		return ui.row { width = INDENT, shrink = false }
	end
	local half = INDENT // 2
	return ui.row {
		width = INDENT,
		height = ROW,
		shrink = false,
		gap = "none",
		align = "start",
		style = { padding = { left = half } },
		ui.column { class = "sb-guide", width = 1, height = if kind == "elbow" then ROW // 2 else ROW },
		kind ~= "pipe" and ui.column { class = "sb-guide-arm", width = INDENT - half - 2, height = ROW // 2 },
	}
end

-- The live helpers of `address`, depth first, each with its guide lines.
local function helpers(children, address: string, prefix: { string }, out: { any }, depth: number)
	local kids = children[address]
	if not kids or depth > MAX_DEPTH then
		return
	end
	for index, kid in kids do
		local last = index == #kids
		local lines = table.clone(prefix)
		table.insert(lines, if last then "elbow" else "tee")
		table.insert(out, { terminal = kid, lines = lines })
		local below = table.clone(prefix)
		table.insert(below, if last then "blank" else "pipe")
		helpers(children, kid.address, below, out, depth + 1)
	end
end

local function sort_button(sort: string)
	local current = SORTS[sort] or SORTS.priority
	return icon_button {
		icon = current.icon,
		size = "small",
		tooltip = `Sort agents: {current.label}`,
		msg = "sort-agents",
	}
end

type Context = {
	state: State,
	app: any,
	workspace: any,
	visible: { [string]: boolean },
	space_names: { [string]: string },
	ours: { any },
	elsewhere: { any },
}

-- The agents list's rows: `ours` under the Agents heading, and `elsewhere`
-- (agents in no pane of this app that no listed agent started).
local function agents_list(ctx: Context): { ours: { any }, elsewhere: { any }, count: number }
	local state, workspace = ctx.state, ctx.workspace
	local core = ctx.app.core
	local now = (core and core.now) or 0
	local priority = state.sort ~= "space"

	-- Live terminals by address, and the helpers each one started.
	local by_address, children = {}, {}
	for _, terminal in (core and core.terminals) or {} do
		by_address[terminal.address] = terminal
	end
	for _, terminal in (core and core.terminals) or {} do
		local parent = terminal.parent
		if parent and parent ~= "" and by_address[parent] then
			children[parent] = children[parent] or {}
			table.insert(children[parent], terminal)
		end
	end

	-- An agent elsewhere that a listed agent started shows under it instead.
	local listed = {}
	for _, terminal in ctx.ours do
		listed[terminal.address] = true
	end
	for _, terminal in ctx.elsewhere do
		listed[terminal.address] = true
	end
	local function nested(terminal): boolean
		local up = by_address[terminal.parent or ""]
		for _ = 1, MAX_DEPTH do
			if not up then
				return false
			elseif listed[up.address] then
				return true
			end
			up = by_address[up.parent or ""]
		end
		return false
	end
	local elsewhere = {}
	for _, terminal in ctx.elsewhere do
		if not nested(terminal) then
			table.insert(elsewhere, terminal)
		end
	end

	-- Each listed agent's status, its helpers, and how urgent the two are.
	local status, subs, urgency = {}, {}, {}
	local function rate(terminal)
		local own = shown_status(terminal, state.seen, ctx.visible)
		local below = {}
		helpers(children, terminal.address, {}, below, 1)
		local level = LOUDNESS[own] or 1
		for _, sub in below do
			level = math.max(level, LOUDNESS[helper_status(sub.terminal)] or 0)
		end
		status[terminal.address], subs[terminal.address], urgency[terminal.address] = own, below, level
	end
	for _, terminal in ctx.ours do
		rate(terminal)
	end
	for _, terminal in elsewhere do
		rate(terminal)
	end
	local function by_urgency(list: { any }): { any }
		local sorted = table.clone(list)
		table.sort(sorted, function(a, b)
			if urgency[a.address] ~= urgency[b.address] then
				return urgency[a.address] > urgency[b.address]
			elseif (a.status_ms or 0) ~= (b.status_ms or 0) then
				return (a.status_ms or 0) > (b.status_ms or 0)
			end
			return a.address < b.address
		end)
		return sorted
	end

	local function when(terminal): any
		local since = terminal.status_ms or 0
		if since <= 0 or now <= 0 then
			return false
		end
		return ui.row { shrink = false, ui.text { age(math.max(0, now - since)), class = "sb-age" } }
	end

	local function click(terminal, turn): (any, string)
		local mine = terminal.ours and terminal.pane ~= "" and workspace.where and workspace.where[terminal.pane]
		if mine then
			return { focus = terminal.pane, address = terminal.address, turn = turn }, "Show its pane"
		end
		return { peek = terminal.address, turn = turn }, "Peek"
	end

	-- One agent: its icon, where it runs and its program, its age and status.
	local function agent_row(terminal, name: string, detail: string?)
		local shown = status[terminal.address]
		local msg, tip = click(terminal, terminal.turn)
		return ui.row {
			key = `a-{terminal.address}`,
			class = { "sb-row", "sb-agent" },
			align = "center",
			msg = msg,
			tooltip = tip,
			ui.icon { name = (AGENT_ICONS[terminal.agent] or "bot") :: any },
			ui.row {
				grow = true,
				gap = "small",
				align = "center",
				ui.text { name, class = if shown == "idle" then "sb-agent-name sb-quiet" else "sb-agent-name", truncate = true },
				detail ~= nil and detail ~= "" and ui.row { grow = true, ui.text { `· {detail}`, class = "sb-program", truncate = true } },
			},
			when(terminal),
			status_dot {
				status = shown,
				chip = shown == "blocked" and not priority,
				spin = shown == "working",
				size = "small",
				tooltip = LABELS[shown],
			},
		}
	end

	-- A helper, indented under the agent that started it.
	local function helper_row(sub)
		local terminal = sub.terminal
		local shown = helper_status(terminal)
		local msg, tip = click(terminal, terminal.turn)
		local cells = {}
		for _, kind in sub.lines do
			table.insert(cells, guide(kind))
		end
		local label = if terminal.name ~= "" then terminal.name else terminal.program
		return ui.row {
			key = `h-{terminal.address}`,
			class = { "sb-row", "sb-helper" },
			align = "center",
			height = ROW,
			msg = msg,
			tooltip = if tip == "Peek" then "Peek at this helper" else tip,
			ui.row { gap = "none", align = "start", height = ROW, shrink = false, table.unpack(cells) },
			ui.icon { name = (AGENT_ICONS[terminal.agent] or "terminal") :: any, size = "small", tone = "muted" },
			ui.row { grow = true, ui.text { label, class = "sb-helper-name", truncate = true } },
			when(terminal),
			status_dot { status = shown, spin = shown == "working", size = "small", tooltip = LABELS[shown] },
		}
	end

	local function with_helpers(out: { any }, terminal, name: string, detail: string?)
		table.insert(out, agent_row(terminal, name, detail))
		for _, sub in subs[terminal.address] do
			table.insert(out, helper_row(sub))
		end
	end

	local function header(out: { any }, id: string, title: string, count: number, dot: string?)
		local folded = state.collapsed[id] == true
		table.insert(out, ui.row {
			key = id,
			class = "sb-group",
			group_header { title = title, status = dot, count = count, collapsed = folded, toggle = { fold = id } },
		})
		return not folded
	end

	local where_of = function(terminal): string
		return ctx.space_names[workspace.where[terminal.pane]] or ""
	end

	local ours = {}
	if priority then
		local groups = {}
		for _, terminal in by_urgency(ctx.ours) do
			local key = SECTIONS[5 - urgency[terminal.address]] or "idle"
			groups[key] = groups[key] or {}
			table.insert(groups[key], terminal)
		end
		for _, key in SECTIONS do
			local members = groups[key]
			if members and header(ours, `st-{key}`, LABELS[key], #members, key) then
				for _, terminal in members do
					with_helpers(ours, terminal, where_of(terminal), terminal.program)
				end
			end
		end
	else
		local by_space = {}
		for _, terminal in ctx.ours do
			local space = workspace.where[terminal.pane]
			by_space[space] = by_space[space] or {}
			table.insert(by_space[space], terminal)
		end
		for _, space in workspace.spaces or {} do
			local members = by_space[space.id]
			if members and header(ours, `sp-{space.id}`, ctx.space_names[space.id] or "", #members, nil) then
				for _, terminal in members do
					-- Under its space's heading, the program says it all.
					with_helpers(ours, terminal, terminal.program, nil)
				end
			end
		end
	end
	if #ours == 0 then
		table.insert(ours, ui.text { "No agent in your spaces yet. Start one in a pane.", class = "sb-empty", wrap = true })
	end

	local others = {}
	local order: { any } = if priority then by_urgency(elsewhere) else elsewhere
	for _, terminal in order do
		with_helpers(others, terminal, base(terminal.cwd), terminal.program)
	end
	return { ours = ours, elsewhere = others, count = #elsewhere }
end

return mod {
	title = "Sidebar",
	subscribe = { "herdr/workspace", "provider/core" },

	styles = {
		[".sb"] = { padding = { 0, 8, 8, 10 }, gap = 2 },
		[".sb-top"] = { padding = { 0, 4, 0, 0 }, gap = 6 },
		[".sb-brand"] = { weight = "semibold" },
		[".sb-head"] = { padding = { 14, 6, 4, 8 }, gap = 6 },
		[".sb-head-text"] = { size = "small", weight = "medium", color = "muted-foreground", case = "upper" },
		[".sb-row:hover"] = { background = "list-hover" },
		[".sb-row:selected"] = { background = "card", shadow = "0 1px 2px #00000018" },
		[".sb-row"] = { transition = "slide-fade fast ease-out", padding = { 6, 8 }, radius = 8, gap = 8 },
		[".sb-name"] = { weight = "medium" },
		[".sb-detail"] = { size = "small", color = "muted-foreground" },
		[".sb-count"] = { size = "small", color = "muted-foreground" },
		[".sb-agent"] = { height = ROW, padding = { 0, 8 }, gap = 9 },
		[".sb-agent-name"] = { weight = "normal" },
		[".sb-quiet"] = { color = "muted-foreground" },
		[".sb-program"] = { color = "muted-foreground" },
		[".sb-age"] = { size = "small", color = "muted-foreground" },
		[".sb-group"] = { padding = { 6, 0, 0, 0 } },
		[".group-header"] = { padding = { 2, 8 } },
		[".group-header-title"] = { size = "small", weight = "medium", color = "muted-foreground" },
		[".group-header-count"] = { size = "small" },
		[".sb-helper"] = { padding = { 0, 8, 0, 14 }, gap = 6 },
		[".sb-helper-name"] = { size = "small", color = "muted-foreground" },
		[".sb-guide"] = { background = "border" },
		[".sb-guide-arm"] = { border = "bottom", border_color = "border" },
		[".sb-empty"] = { size = "small", color = "muted-foreground", padding = { 4, 8 } },
		[".sb-problem"] = { size = "small", color = "danger", padding = { 4, 8 } },
	},

	init = function(previous: State?): State
		return {
			workspace = previous and previous.workspace or nil,
			seen = previous and previous.seen or {},
			collapsed = previous and previous.collapsed or {},
			blocked = nil,
			sort = if previous and previous.sort == "space" then "space" else "priority",
		}
	end,

	update = function(state: State, msg)
		if type(msg) == "table" and msg.topic == "herdr/workspace" then
			state.workspace = msg.value
			return state
		elseif type(msg) == "table" and msg.topic == "provider/core" then
			-- Remember the agent that has needed you the shortest while, for ⌘J.
			local blocked, since = nil, -1
			for _, terminal in (msg.value and msg.value.terminals) or {} do
				if terminal.status == "blocked" and (terminal.status_ms or 0) >= since then
					blocked, since = terminal.address, terminal.status_ms or 0
				end
			end
			state.blocked = blocked
			-- Forget what was seen of terminals that are gone.
			local live = {}
			for _, terminal in (msg.value and msg.value.terminals) or {} do
				live[terminal.address] = true
			end
			for address in state.seen do
				if not live[address] then
					state.seen[address] = nil
				end
			end
			return state
		elseif msg == "peek-blocked" then
			if state.blocked then
				return state, { { to = "peek", msg = { open = state.blocked } } }
			end
			return
		elseif type(msg) == "table" and msg.peek then
			state.seen[msg.peek] = msg.turn
			return state, { { to = "peek", msg = { open = msg.peek } } }
		elseif type(msg) == "table" and msg.focus then
			state.seen[msg.address] = msg.turn
			return state, { { to = "workspace", msg = { focus_pane = msg.focus } } }
		elseif msg == "sort-agents" then
			state.sort = (SORTS[state.sort] or SORTS.priority).next
			return state
		elseif type(msg) == "table" and msg.fold then
			state.collapsed[msg.fold] = if state.collapsed[msg.fold] then nil else true
			return state
		end
	end,

	view = function(state: State, app)
		local core = app.core
		local workspace = state.workspace or { spaces = {}, visible = {}, where = {} }
		local visible = {}
		for _, pane in workspace.visible or {} do
			visible[pane] = true
		end
		local mac = app.window.platform == "macos"
		local hidden = app.window.titlebar == "hidden"

		local by_pane = {}
		local ours, elsewhere = {}, {}
		for _, terminal in (core and core.terminals) or {} do
			if terminal.ours and terminal.pane ~= "" then
				by_pane[terminal.pane] = terminal
			end
			if terminal.is_agent then
				-- One of ours only while its pane still is: a closed pane's
				-- shell may outlive it by a tick.
				if terminal.ours and workspace.where and workspace.where[terminal.pane] then
					table.insert(ours, terminal)
				elseif not terminal.ours then
					table.insert(elsewhere, terminal)
				end
			end
		end

		local space_names = {}
		local rows = {}
		for index, space in workspace.spaces or {} do
			local first = by_pane[space.panes[1] or ""]
			local label = if space.name ~= "" then space.name else base(if first then first.cwd else space.cwd)
			space_names[space.id] = label
			local loud, level = nil, 0
			local agents = 0
			for _, pane in space.panes do
				local terminal = by_pane[pane]
				if terminal and terminal.is_agent then
					agents += 1
					local status = shown_status(terminal, state.seen, visible)
					if (LOUDNESS[status] or 0) > level then
						loud, level = status, LOUDNESS[status]
					end
				end
			end
			table.insert(rows, ui.row {
				key = space.id,
				class = "sb-row",
				align = "center",
				selected = space.id == workspace.space,
				msg = { select_space = space.id },
				to = "workspace",
				context = { rename_space = space.id },
				tooltip = "Right-click to rename",
				ui.icon { name = "folder", tone = if space.id == workspace.space then "accent" else "muted" },
				ui.column {
					grow = true,
					gap = "none",
					ui.text { label, class = "sb-name", truncate = true },
					(function()
						local cwd = if first then first.cwd else space.cwd
						return if cwd ~= label then ui.text { cwd, class = "sb-detail", truncate = true } else nil
					end)(),
				},
				loud and status_dot { status = loud, size = "small", tooltip = `{agents} agent{if agents == 1 then "" else "s"}` },
				space.tabs > 1 and ui.text { tostring(space.tabs), class = "sb-count" },
				#workspace.spaces > 1 and icon_button { icon = "close", size = "small", tooltip = "Close space and its terminals", msg = { close_space = space.id }, to = "workspace" },
			})
		end

		local function section(id: string, title: string, count: number?, action: any, children: { any })
			local open = not state.collapsed[id]
			return ui.column {
				gap = "none",
				ui.row {
					class = "sb-head",
					align = "center",
					msg = { fold = id },
					ui.icon { name = if open then "chevron-down" else "chevron-right", tone = "muted", size = "small" },
					ui.text { title, class = "sb-head-text" },
					count and count > 0 and ui.text { tostring(count), class = "sb-count" },
					ui.spacer(),
					action,
				},
				open and ui.column { gap = "none", table.unpack(children) },
			}
		end

		local list = agents_list {
			state = state,
			app = app,
			workspace = workspace,
			visible = visible,
			space_names = space_names,
			ours = ours,
			elsewhere = elsewhere,
		}

		local problem = core and not core.ok and ui.text { core.error, class = "sb-problem", wrap = true }

		return ui.column {
			grow = true,
			gap = "none",
			ui.row {
				class = "sb-top",
				drag = true,
				height = 46,
				shrink = false,
				align = "center",
				hidden and mac and not app.window.fullscreen and ui.row { width = 72, shrink = false },
				ui.spacer(),
				icon_button { icon = "panel-left", tooltip = "Hide sidebar (⌘B)", msg = "toggle-left", to = "shell" },
			},
			ui.column {
				class = "sb",
				grow = true,
				scroll = true,
				section("spaces", "Spaces", nil,
					icon_button { icon = "plus", size = "small", tooltip = "New space (⌘N)", msg = "new-space", to = "workspace" },
					rows),
				section("agents", "Agents", #ours, sort_button(state.sort), list.ours),
				list.count > 0 and section("elsewhere", "Elsewhere", list.count, nil, list.elsewhere),
				problem,
			},
		}
	end,
}
