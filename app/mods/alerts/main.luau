-- Alerts: a sound and a system notification when one of this app's agents
-- turns to Needs you (`blocked`) or Done while you are not looking at it,
-- that is while the window is not active, or its pane is not on screen
-- (another tab or space) and no peek shows it. Agents elsewhere alert too
-- when Settings > Alerts says so.
--
-- It watches `app.core` (the `provider/core` topic) and remembers each
-- agent's last status, so it alerts only on a change it saw happen: never
-- for what the first look after a launch or reload finds, nor for a
-- terminal it had not seen before, and once per change. Settings shares
-- the switches on `moonmux/alerts`; the shell mounts this mod with
-- `{ active = app.window.active }` as props, since `update` cannot read
-- `app`. Clicking a notification (`app/notification`) goes to the agent as
-- a click on its sidebar row does: its pane, or a peek.

type Agent = { status: string, turn: number, pane: string?, program: string, cwd: string }
type State = {
	-- Each agent's last status, by terminal address; nil until the first
	-- look, which alerts for nothing.
	known: { [string]: Agent }?,
	enabled: boolean,
	elsewhere: boolean,
	-- From `moonmux/workspace`: the panes on screen, each pane's space, and
	-- the spaces' names.
	visible: { [string]: boolean },
	where: { [string]: string },
	names: { [string]: string },
}

local TITLES = { blocked = "needs you", done = "finished" }

-- A notification's `id` must be 1 to 128 of a-z A-Z 0-9 - _ . : /.
local function id_of(address: string): string?
	if #address <= 128 and string.match(address, "^[%w%-_%.:/]+$") then
		return address
	end
	return nil
end

local function base(path: string): string
	return string.match(path, "([^/]+)/*$") or path
end

local function workspace(state: State, value: any)
	state.visible, state.where, state.names = {}, {}, {}
	for _, pane in (value and value.visible) or {} do
		state.visible[pane] = true
	end
	for pane, space in (value and value.where) or {} do
		state.where[pane] = space
	end
	for _, space in (value and value.spaces) or {} do
		state.names[space.id] = if space.name ~= "" then space.name else base(space.cwd or "")
	end
end

-- The alerts for one look at herdr-core, and what it now knows.
local function look(state: State, core: any, active: boolean): { any }
	if not core or not core.ok then
		-- A failed look says nothing about the agents: keep what was known.
		return {}
	end
	local before = state.known
	local known: { [string]: Agent } = {}
	local peeked = core.peek and core.peek.address
	local sends: { any } = {}
	for _, terminal in core.terminals or {} do
		if not terminal.is_agent then
			continue
		end
		local pane = if terminal.ours and terminal.pane ~= "" then terminal.pane else nil
		local agent: Agent = {
			status = terminal.status or "",
			turn = terminal.turn or 0,
			pane = pane,
			program = if terminal.program ~= "" then terminal.program else "An agent",
			cwd = terminal.cwd or "",
		}
		known[terminal.address] = agent
		local last = before and before[terminal.address]
		local verb = TITLES[agent.status]
		local changed = last ~= nil and (last.status ~= agent.status or last.turn ~= agent.turn)
		if not changed or not verb or not state.enabled then
			continue
		end
		local space = pane and state.where[pane]
		if pane and not space then
			continue -- a closed pane's shell, gone in a tick
		elseif not pane and not state.elsewhere then
			continue
		end
		local on_screen = (pane ~= nil and state.visible[pane] == true) or peeked == terminal.address
		if active and on_screen then
			continue
		end
		local where = if space then state.names[space] or "" else "Elsewhere"
		local body = if where ~= "" and agent.cwd ~= "" then `{where} · {agent.cwd}` else where .. agent.cwd
		table.insert(sends, { to = "app:sound", msg = "default" })
		table.insert(sends, { to = "app:notify", msg = { title = `{agent.program} {verb}`, body = body, id = id_of(terminal.address) } })
	end
	state.known = known
	return sends
end

-- Where a click on the notification for `address` goes, as the sidebar
-- would on a click of its row.
local function open(state: State, address: string): { any }
	local agent = state.known and state.known[address]
	if not agent then
		return {}
	elseif agent.pane and state.where[agent.pane] then
		return { { to = "sidebar", msg = { focus = agent.pane, address = address, turn = agent.turn } } }
	end
	return { { to = "sidebar", msg = { peek = address, turn = agent.turn } } }
end

return mod {
	title = "Alerts",
	subscribe = { "provider/core", "moonmux/workspace", "moonmux/alerts", "app/notification" },

	init = function(): State
		-- Nothing known: the first look after a launch or reload is quiet.
		return { known = nil, enabled = true, elsewhere = false, visible = {}, where = {}, names = {} }
	end,

	update = function(state: State, msg, props)
		if type(msg) ~= "table" then
			return
		end
		if msg.topic == "provider/core" then
			local sends = look(state, msg.value, props ~= nil and props.active == true)
			return state, sends
		elseif msg.topic == "moonmux/workspace" then
			workspace(state, msg.value)
			return state
		elseif msg.topic == "moonmux/alerts" then
			local value = msg.value or {}
			state.enabled = value.enabled ~= false
			state.elsewhere = value.elsewhere == true
			return state
		elseif msg.topic == "app/notification" then
			local value = msg.value or {}
			if value.clicked and type(value.id) == "string" then
				return state, open(state, value.id)
			end
		end
	end,

	view = function()
		return ui.row { width = 0, height = 0 }
	end,
}
