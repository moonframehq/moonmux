-- Herdr's shell: the sidebar (spaces, then the agents in them and
-- elsewhere) on the left, the chosen space's tabs and split panes in a card
-- on the right, a status line under both, and the peek sheet over them;
-- over all of them Settings, the command palette and the Welcome.
-- The title bar is hidden: its traffic lights sit over the sidebar, or
-- over the tab strip while the sidebar is folded.

-- The menus after the app menu (About, Quit); "View" merges into the
-- host's. A typed local, so items with `msg` and `command` mix.
local MENUS: { { title: string, items: { any } } } = {
	{ title = "File", items = {
		{ label = "New Space…", msg = "new-space", to = "workspace", shortcut = "secondary-n" },
		{ label = "New Tab", msg = "new-tab", to = "workspace", shortcut = "secondary-t" },
		"-",
		{ label = "Split Right", msg = "split-right", to = "workspace", shortcut = "secondary-d" },
		{ label = "Split Down", msg = "split-down", to = "workspace", shortcut = "secondary-shift-d" },
		"-",
		{ label = "Close Pane", msg = "close-pane", to = "workspace", shortcut = "secondary-w" },
		{ label = "Close Tab", msg = "close-tab", to = "workspace", shortcut = "secondary-shift-w" },
		"-",
		{ label = "Settings…", msg = "open", to = "settings", shortcut = "secondary-," },
	} },
	{ title = "View", items = {
		{ label = "Command Palette…", msg = "open", to = "palette", shortcut = "secondary-shift-p" },
		"-",
		{ label = "Show or Hide Sidebar", msg = "toggle-left", shortcut = "secondary-b" },
		{ label = "Themes…", command = "show-themes" },
		"-",
		{ label = "Next Tab", msg = "next-tab", to = "workspace", shortcut = "secondary-alt-right" },
		{ label = "Previous Tab", msg = "prev-tab", to = "workspace", shortcut = "secondary-alt-left" },
		{ label = "Next Space", msg = "next-space", to = "workspace", shortcut = "secondary-alt-down" },
		{ label = "Previous Space", msg = "prev-space", to = "workspace", shortcut = "secondary-alt-up" },
		"-",
		{ label = "Tab 1", msg = { tab_index = 1 }, to = "workspace", shortcut = "secondary-1" },
		{ label = "Tab 2", msg = { tab_index = 2 }, to = "workspace", shortcut = "secondary-2" },
		{ label = "Tab 3", msg = { tab_index = 3 }, to = "workspace", shortcut = "secondary-3" },
		{ label = "Tab 4", msg = { tab_index = 4 }, to = "workspace", shortcut = "secondary-4" },
		{ label = "Tab 5", msg = { tab_index = 5 }, to = "workspace", shortcut = "secondary-5" },
		{ label = "Tab 6", msg = { tab_index = 6 }, to = "workspace", shortcut = "secondary-6" },
		{ label = "Tab 7", msg = { tab_index = 7 }, to = "workspace", shortcut = "secondary-7" },
		{ label = "Tab 8", msg = { tab_index = 8 }, to = "workspace", shortcut = "secondary-8" },
		{ label = "Last Tab", msg = { tab_index = -1 }, to = "workspace", shortcut = "secondary-9" },
	} },
	{ title = "Agents", items = {
		{ label = "Peek at the Agent That Needs You", msg = "peek-blocked", to = "sidebar", shortcut = "secondary-j" },
		{ label = "Close Peek", msg = "close", to = "peek" },
	} },
	{ title = "Window", items = {
		{ label = "Minimize", command = "minimize", shortcut = "secondary-m" },
		{ label = "Zoom", command = "maximize" },
	} },
	{ title = "Help", items = {
		{ label = "Welcome to Herdr", msg = "open", to = "welcome" },
	} },
}

return mod {
	window = { titlebar = "hidden", traffic_lights = { x = 16, y = 18 }, size = { width = 1400, height = 880 } },

	about = {
		version = "0.1",
		description = "Terminals and coding agents in spaces, kept running by herdr-core or tmux.",
		website = "https://github.com/penso/herdr-moonframe",
		copyright = "MIT licensed",
	},

	menus = MENUS,

	subscribe = { "herdr/layout" },

	init = function(previous)
		local state = { left = if previous then previous.left ~= false else true, status = true }
		-- The palette lists what the menus offer.
		local sends: { any } = { { to = "workspace", msg = { sidebar = state.left } }, { to = "palette", msg = { menus = MENUS } } }
		return state, sends
	end,

	update = function(state, msg)
		if type(msg) == "table" and msg.topic == "herdr/layout" then
			state.status = not (msg.value and msg.value.status == "hidden")
			return state
		end
		if msg == "toggle-left" then
			state.left = not state.left
			-- The tab strip makes room for the traffic lights while the
			-- sidebar is folded.
			return state, { { to = "workspace", msg = { sidebar = state.left } } }
		end
	end,

	view = function(state)
		local body = ui.hsplit {
			id = "body",
			divider = false,
			ui.pane { size = 268, min = 210, max = 420, fixed = true, visible = state.left, class = "side", ui.mount "sidebar" },
			ui.pane { class = "middle", ui.mount "workspace" },
		}
		return ui.column {
			grow = true,
			gap = "none",
			body,
			state.status and ui.mount "statusbar",
			ui.mount "peek",
			ui.mount "settings",
			ui.mount "palette",
			ui.mount "welcome",
		}
	end,
}
