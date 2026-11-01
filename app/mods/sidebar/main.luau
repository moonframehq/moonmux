-- The sidebar: the spaces (from the workspace's `herdr/workspace` topic),
-- then every agent herdr-core runs (`app.core`): first those in this app's
-- panes, then those elsewhere (another app, `hc run`, an agent's workers).
--
-- An agent is Working, Blocked (it needs you), Done (it finished while you
-- looked elsewhere) or Idle. A done agent you have looked at (its pane is
-- on screen, or you peeked at it since it finished) shows as Idle.
-- Clicking one of ours shows its pane; clicking any other peeks at it.

local icon_button = use "icon-button"
local status_dot = use "status-dot"

type State = {
	workspace: any,
	seen: { [string]: number },
	collapsed: { [string]: boolean? },
	blocked: string?,
}

local AGENT_ICONS = {
	["claude-code"] = "agent-claude", claude = "agent-claude", codex = "agent-codex", gemini = "agent-gemini",
	cursor = "agent-cursor", ["cursor-agent"] = "agent-cursor", copilot = "agent-copilot",
	opencode = "agent-opencode", amp = "agent-amp", droid = "agent-droid", kiro = "agent-kiro",
	qwen = "agent-qwen", grok = "agent-grok",
}

local LABELS = { working = "Working", blocked = "Needs you", done = "Done", idle = "Idle" }
local LOUDNESS = { blocked = 4, working = 3, done = 2, idle = 1 }

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
		[".sb-empty"] = { size = "small", color = "muted-foreground", padding = { 4, 8 } },
		[".sb-problem"] = { size = "small", color = "danger", padding = { 4, 8 } },
	},

	init = function(previous: State?): State
		return {
			workspace = previous and previous.workspace or nil,
			seen = previous and previous.seen or {},
			collapsed = previous and previous.collapsed or {},
			blocked = nil,
		}
	end,

	update = function(state: State, msg)
		if type(msg) == "table" and msg.topic == "herdr/workspace" then
			state.workspace = msg.value
			return state
		elseif type(msg) == "table" and msg.topic == "provider/core" then
			-- Remember the newest blocked agent, for ⌘J.
			local blocked = nil
			for _, terminal in (msg.value and msg.value.terminals) or {} do
				if terminal.status == "blocked" then
					blocked = terminal.address
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

		local function agent_row(terminal, mine: boolean)
			local status = shown_status(terminal, state.seen, visible)
			local where = if mine then (space_names[workspace.where[terminal.pane]] or "") else base(terminal.cwd)
			return ui.row {
				key = `a-{terminal.address}`,
				class = "sb-row",
				align = "center",
				msg = if mine
					then { focus = terminal.pane, address = terminal.address, turn = terminal.turn }
					else { peek = terminal.address, turn = terminal.turn },
				tooltip = if mine then "Show its pane" else "Peek",
				ui.icon { name = (AGENT_ICONS[terminal.agent] or "bot") :: any },
				ui.column {
					grow = true,
					gap = "none",
					ui.text { terminal.agent, class = "sb-name", truncate = true },
					ui.text { `{terminal.address} · {where}`, class = "sb-detail", truncate = true },
				},
				status_dot { status = status, chip = status == "blocked", spin = status == "working", size = "small", tooltip = LABELS[status] },
			}
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

		local agent_rows = {}
		for _, terminal in ours do
			table.insert(agent_rows, agent_row(terminal, true))
		end
		if #agent_rows == 0 then
			table.insert(agent_rows, ui.text { "No agent in your spaces yet. Start one in a pane.", class = "sb-empty", wrap = true })
		end
		local other_rows = {}
		for _, terminal in elsewhere do
			table.insert(other_rows, agent_row(terminal, false))
		end

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
				section("agents", "Agents", #ours, nil, agent_rows),
				#other_rows > 0 and section("elsewhere", "Elsewhere", #other_rows, nil, other_rows),
				problem,
			},
		}
	end,
}
