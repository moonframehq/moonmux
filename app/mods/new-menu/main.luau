-- The tab strip's "+": a menu to open a new tab with a shell (⌘T) or an
-- agent, or a new space (⌘N), as herdr-app's new-menu. The workspace
-- mounts it in its strip, and this mod draws the button and the popover.
--
-- The agents are the ones the `core` provider can start (`app.core.programs`),
-- installed ones first, numbered 1-9 while the menu is open; those whose
-- executable is not found are greyed out as "not installed". Starting one
-- tells the workspace `{ start_agent = key }`: it opens a tab and has
-- `provider:core` type the agent's command into it once its shell is up.

local icon_button = use "icon-button"
local kbd = use "kbd"
local program_icon = use "program-icon"

type Program = { key: string, name: string, installed: boolean }
type State = {
	open: boolean,
	-- Counts openings: each opening gets a fresh, empty key catcher.
	opened: number,
	-- The keyboard-highlighted row's id.
	highlight: string?,
	programs: { Program },
}

-- Installed agents first, in the provider's order, then the rest.
local function ordered(programs: { Program }): { Program }
	local list = {}
	for _, program in programs do
		if program.installed then
			table.insert(list, program)
		end
	end
	for _, program in programs do
		if not program.installed then
			table.insert(list, program)
		end
	end
	return list
end

-- The ids of the rows that can be picked, top to bottom.
local function choices(state: State): { string }
	local ids = { "terminal" }
	for _, program in ordered(state.programs) do
		if program.installed then
			table.insert(ids, program.key)
		end
	end
	table.insert(ids, "space")
	return ids
end

local function move(state: State, step: number)
	local ids = choices(state)
	local at = table.find(ids, state.highlight or "") or (if step > 0 then 0 else 1)
	state.highlight = ids[(at - 1 + step) % #ids + 1]
end

-- Closes the menu and does what row `id` says.
local function pick(state: State, id: string): (State, { any })
	state.open = false
	state.highlight = nil
	local msg: any = nil
	if id == "terminal" then
		msg = "new-tab"
	elseif id == "space" then
		msg = "new-space"
	end
	for _, program in state.programs do
		if program.key == id and program.installed then
			msg = { start_agent = id }
		end
	end
	return state, if msg then { { to = "workspace", msg = msg } } else {}
end

return mod {
	title = "New",
	subscribe = { "provider/core" },

	styles = {
		[".nm"] = { padding = 5, radius = 10 },
		[".nm-row"] = { height = 31, padding = { 0, 9 }, gap = 10, radius = 7 },
		[".nm-row:hover"] = { background = "list-hover" },
		[".nm-row:selected"] = { background = "list-hover" },
		[".nm-off"] = { opacity = 0.45 },
		[".nm-hint"] = { size = "small", color = "muted-foreground" },
		[".nm-heading"] = { padding = { 8, 9, 4, 9 }, size = "small", color = "muted-foreground" },
		[".nm-rule"] = { margin = { 5, 4 } },
		[".nm-plus"] = { width = 14 },
		[".nm-keys"] = { opacity = 0, padding = 0, border = "none" },
	},

	init = function(previous: State?): State
		return {
			open = false,
			opened = previous and previous.opened or 0,
			highlight = nil,
			programs = previous and previous.programs or {},
		}
	end,

	update = function(state: State, msg)
		if msg == "toggle" then
			state.open = not state.open
			state.highlight = nil
			if state.open then
				state.opened += 1
			end
			return state
		elseif msg == "close" then
			state.open = false
			return state
		elseif msg == "down" or msg == "up" then
			move(state, if msg == "down" then 1 else -1)
			return state
		elseif msg == "enter" then
			return pick(state, state.highlight or "terminal")
		end
		if type(msg) ~= "table" then
			return
		end
		if msg.topic == "provider/core" then
			local programs = (msg.value and msg.value.programs) or {}
			local same = #programs == #state.programs
			for index, program in programs do
				local was = state.programs[index]
				same = same and was ~= nil and was.key == program.key and was.installed == program.installed
			end
			if same then
				return
			end
			state.programs = programs
			return state
		elseif msg.pick then
			return pick(state, msg.pick)
		elseif msg.msg == "typed" and state.open then
			-- A digit typed while the menu is open starts that agent.
			local digit = tonumber(string.sub(msg.value or "", -1))
			local installed = {}
			for _, program in ordered(state.programs) do
				if program.installed then
					table.insert(installed, program)
				end
			end
			local program = digit and installed[digit]
			if program then
				return pick(state, program.key)
			end
			return
		end
		return
	end,

	view = function(state: State, app)
		local function row(id: string, icon: any, label: string, hint: any, enabled: boolean)
			return ui.row {
				class = if enabled then "nm-row" else { "nm-row", "nm-off" },
				align = "center",
				shrink = false,
				selected = enabled and state.highlight == id,
				msg = if enabled then { pick = id } else nil,
				icon,
				ui.row { grow = true, ui.text { label, truncate = true } },
				hint,
			}
		end

		local rows = {}
		local number = 0
		for _, program in ordered(state.programs) do
			local hint
			if program.installed then
				number += 1
				hint = number <= 9 and ui.text { tostring(number), class = "nm-hint" }
			else
				hint = ui.text { "not installed", class = "nm-hint" }
			end
			table.insert(rows, row(program.key, program_icon { program = program.key, dark = app.dark }, program.name, hint, program.installed))
		end

		local content = ui.column {
			gap = "none",
			row("terminal", program_icon { program = "shell" }, "Terminal", kbd { keys = "secondary-t", small = true, plain = true }, true),
			ui.text { "Start an agent", class = "nm-heading" },
			#rows == 0 and ui.text { "Checking installed agents…", class = { "nm-heading" } },
			table.unpack(rows),
		}
		local menu = ui.column {
			gap = "none",
			content,
			ui.column { class = "nm-rule", ui.divider() },
			row("space", ui.row { class = "nm-plus", justify = "center", ui.icon { "plus" } }, "New space", kbd { keys = "secondary-n", small = true, plain = true }, true),
			-- Catches the digits: keys on a popover can't take what is typed.
			-- A fresh field each opening, so it starts empty.
			state.open and ui.row {
				float = "top-left",
				width = 1,
				height = 1,
				ui.input { id = `keys-{state.opened}`, class = "nm-keys", width = 1, focus = true, change = "typed" },
			},
		}

		return ui.popover {
			open = state.open,
			dismiss = "close",
			side = "bottom",
			align = "start",
			width = 260,
			class = "nm",
			keys = { up = "up", down = "down", enter = "enter" } :: any,
			content = menu,
			icon_button { icon = "plus", tooltip = "New tab or agent (⌘T for a shell)", msg = "toggle", active = state.open },
		}
	end,
}
