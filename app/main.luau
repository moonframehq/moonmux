-- Herdr's shell: the sidebar (spaces, then the agents in them and
-- elsewhere) on the left, the chosen space's tabs and split panes in a card
-- on the right, the Files or Changes panel beside it when shown, a status
-- line under all, and the peek sheet over them; over all of them Settings,
-- the command palette and the Welcome; and the alerts, which draw nothing
-- but need to know whether the window is active.
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
		{ label = "Show or Hide Files", msg = { panel = "files" } },
		{ label = "Show or Hide Changes", msg = { panel = "changes" } },
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

-- The right-hand panels: mods/files and mods/changes.
local PANELS: { [string]: boolean } = { files = true, changes = true }

type State = {
	left: boolean,
	status: boolean,
	-- The right-hand panel shown, "files" or "changes", if any.
	panel: string?,
	-- The last one shown, kept drawn while its pane slides away.
	last: string?,
}

return mod {
	window = { titlebar = "hidden", traffic_lights = { x = 16, y = 18 }, size = { width = 1400, height = 880 } },

	about = {
		version = "0.1",
		description = "Terminals and coding agents in spaces, kept running by herdr-core.",
		website = "https://github.com/penso/herdr-moonframe",
		copyright = "MIT licensed",
	},

	menus = MENUS,

	subscribe = { "herdr/layout" },

	init = function(previous: any): (State, { any })
		local state: State = {
			left = if previous then previous.left ~= false else true,
			status = true,
			panel = if previous and PANELS[previous.panel] then previous.panel else nil,
			last = if previous and PANELS[previous.last] then previous.last else nil,
		}
		return state, {
			{ to = "workspace", msg = { sidebar = state.left } },
			{ to = "workspace", msg = { panel = state.panel or false } },
			-- The palette lists what the menus offer.
			{ to = "palette", msg = { menus = MENUS } },
		}
	end,

	update = function(state: State, msg)
		if type(msg) == "table" and msg.topic == "herdr/layout" then
			state.status = not (msg.value and msg.value.status == "hidden")
			return state
		end
		if type(msg) == "table" and PANELS[msg.panel] then
			-- One panel at a time; its own button again hides it.
			state.panel = if state.panel == msg.panel then nil else msg.panel
			state.last = state.panel or state.last
			return state, { { to = "workspace", msg = { panel = state.panel or false } } }
		end
		if msg == "toggle-left" then
			state.left = not state.left
			-- The tab strip makes room for the traffic lights while the
			-- sidebar is folded.
			return state, { { to = "workspace", msg = { sidebar = state.left } } }
		end
	end,

	view = function(state: State, app)
		local panel: string? = state.panel or state.last
		local body = ui.hsplit {
			id = "body",
			divider = false,
			ui.pane { size = 268, min = 210, max = 420, fixed = true, visible = state.left, class = "side", ui.mount "sidebar" },
			ui.pane { class = "middle", ui.mount "workspace" },
			-- Each panel is an instance, so a hidden one sleeps: it neither
			-- runs nor asks its provider for anything.
			ui.pane {
				size = 368,
				min = 280,
				max = 900,
				fixed = true,
				lazy = true,
				visible = state.panel ~= nil,
				surface = "background",
				panel and ui.mount { panel :: string, id = "right" },
			},
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
			ui.mount { "alerts", id = "main", props = { active = app.window.active } },
		}
	end,
}
