-- The command palette (⌘⇧P, or the status line's "⌘⇧P commands"): every
-- action the menus offer, by name and with its shortcut, filtered as you
-- type; Enter runs one. The shell hands it its menus as `{ menus = ... }`,
-- so the two never drift apart.
--
-- A menu item that runs an app command can't be run from here: Moonframe
-- runs commands only from a control, never from `update`. Themes… opens
-- Settings > Look instead, where its button is; Minimize and Zoom are left
-- to the menu.

local command_palette = use "command-palette"

type Entry = { label: string, group: string, shortcut: string?, detail: string?, to: string?, msg: any }
type State = { open: boolean, menus: { any } }

-- The palette's entries: each menu item it can run, by menu.
local function entries(menus: { any }): { Entry }
	local list: { Entry } = {}
	for _, menu in menus do
		for _, item in menu.items or {} do
			if type(item) ~= "table" or item.to == "palette" then
				continue
			end
			if item.command == "show-themes" then
				table.insert(list, { label = item.label, group = menu.title, detail = "in Settings › Look", to = "settings", msg = { open = "look" } })
			elseif item.command == nil and item.msg ~= nil then
				table.insert(list, { label = item.label, group = menu.title, shortcut = item.shortcut, to = item.to, msg = item.msg })
			end
		end
	end
	return list
end

return mod {
	title = "Command palette",

	init = function(previous: State?): State
		return { open = false, menus = previous and previous.menus or {} }
	end,

	update = function(state: State, msg)
		if msg == "open" then
			state.open = true
		elseif msg == "close" then
			state.open = false
		elseif type(msg) == "table" and msg.menus then
			state.menus = msg.menus
		elseif type(msg) == "table" and msg.run then
			state.open = false
			local entry = entries(state.menus)[tonumber(msg.run) or 0]
			if entry then
				return state, { { to = entry.to or "shell", msg = entry.msg } }
			end
		else
			return
		end
		return state
	end,

	view = function(state: State)
		local commands = {}
		for index, entry in entries(state.menus) do
			table.insert(commands, {
				id = tostring(index),
				label = entry.label,
				group = entry.group,
				shortcut = entry.shortcut,
				detail = entry.detail,
			})
		end
		return command_palette {
			id = "main",
			open = state.open,
			dismiss = "close",
			commands = commands,
			placeholder = "Type a command…",
		}
	end,
}
