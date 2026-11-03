-- Settings: a window of its own (Moonframe's `app:window`) with its pages in
-- a nav on the left (Look, Terminals, Machines, Alerts, Updates, Privacy)
-- and Make it yours under them. The shell does not mount it.
-- ⌘, (File > Settings…), the sidebar's Settings row and the title bar's
-- layout button send it `"open"` or `{ open = "machines" }`: it goes to the
-- page and opens its window, or raises it. Done, Escape and ⌘W close it.
--
-- Look holds the layout choices (tab style, pane headers, status line),
-- kept on the topic `moonmux/layout` as { tabs, headers, status }, which the
-- workspace and the shell draw to match, and the appearance (System, Light
-- or Dark) and a light and a dark theme, which Moonframe keeps
-- (`app.appearance`). Terminals holds the backend new panes run on (Auto,
-- herdr-core or tmux: only those built in and installed, from
-- `app.core.backends`), kept on the topic `moonmux/backend` as { choice },
-- which the workspace reads when it opens a pane. Alerts are shared on
-- `moonmux/alerts` as { enabled, elsewhere } for mods/alerts. Machines lists
-- `app.core.machines` (`hc machine ls`), read-only.

local settings_sheet = use "settings-sheet"
local segmented = use "segmented"
local switch = use "switch"
local status_dot = use "status-dot"
local copy_field = use "copy-field"
local appearance = use "appearance"
local theme_cards = use "theme-cards"

type Layout = { tabs: string, headers: string, status: string }
type State = {
	page: string,
	-- Sounds and notifications when an agent needs you or finishes (on by
	-- default), and for agents elsewhere too (off by default).
	alerts: boolean,
	elsewhere: boolean,
	layout: Layout,
	-- The backend new panes run on: "auto", "herdr-core" or "tmux".
	backend: string,
	copied: boolean,
	-- Why the releases page did not open, from the core provider's reply.
	error: string?,
}

local DEFAULTS: Layout = { tabs = "pills", headers = "always", status = "shown" }
local PAGES = {
	{ id = "look", label = "Look", icon = "palette" },
	{ id = "terminals", label = "Terminals", icon = "terminal" },
	{ id = "machines", label = "Machines", icon = "server" },
	{ id = "alerts", label = "Alerts", icon = "bell" },
	{ id = "updates", label = "Updates", icon = "refresh-cw" },
	{ id = "privacy", label = "Privacy", icon = "shield" },
}
local TITLES = { look = "Look", terminals = "Terminals", machines = "Machines", alerts = "Alerts", updates = "Updates", privacy = "Privacy" }
local ADD_MACHINE = "hc machine add <host>"
local DONE = { label = "Done", msg = "close", variant = "primary" }
local OPEN_WINDOW: any = { to = "app:window", msg = { open = "settings" } }
local CLOSE_WINDOW: any = { to = "app:window", msg = { close = "settings" } }

local BACKENDS = { auto = true, ["herdr-core"] = true, tmux = true }

local function announce(layout: Layout)
	return { to = "topic:moonmux/layout", msg = { tabs = layout.tabs, headers = layout.headers, status = layout.status } }
end

local function announce_backend(choice: string)
	return { to = "topic:moonmux/backend", msg = { choice = choice } }
end

local function announce_alerts(state: State)
	return { to = "topic:moonmux/alerts", msg = { enabled = state.alerts, elsewhere = state.elsewhere } }
end

-- Goes to `page`, reading the machines again for their page.
local function select(state: State, page: string): (State, { any })
	state.page = if TITLES[page] then page else "look"
	state.copied = false
	state.error = nil
	if state.page == "machines" then
		return state, { { to = "provider:core", msg = { refresh = "machines" } } }
	end
	return state, {}
end

-- Opens the window on `page`, or raises it there.
local function show(state: State, page: string): (State, { any })
	local sends
	state, sends = select(state, page)
	table.insert(sends, OPEN_WINDOW)
	return state, sends
end

return mod {
	title = "Settings",
	window = { size = { width = 860, height = 600 }, min_size = { width = 640, height = 440 } },
	subscribe = { "moonmux/layout", "moonmux/backend" },

	styles = {
		-- The page fills the window: no card edge of its own.
		[".settings-sheet"] = { border = "none", radius = 0 },
		[".st-note"] = { size = "small", color = "muted-foreground" },
		[".st-machine"] = { padding = { 10, 0 }, gap = 10, border = "bottom", border_color = "border" },
		[".st-machine-detail"] = { size = "small", color = "muted-foreground" },
		[".st-error"] = { size = "small", color = "danger" },
		[".st-feedback"] = { padding = { 6, 8 }, radius = 7, gap = 8, color = "muted-foreground" },
		[".st-feedback:hover"] = { background = "list-hover", color = "foreground" },
	},

	init = function(previous: State?): (State, { any })
		local layout = previous and previous.layout or DEFAULTS
		local state: State = {
			page = previous and previous.page or "look",
			alerts = if previous then previous.alerts ~= false else true,
			elsewhere = previous ~= nil and previous.elsewhere == true,
			layout = { tabs = layout.tabs or DEFAULTS.tabs, headers = layout.headers or DEFAULTS.headers, status = layout.status or DEFAULTS.status },
			backend = if previous and BACKENDS[previous.backend] then previous.backend else "auto",
			copied = false,
			error = nil,
		}
		return state, { announce_alerts(state) }
	end,

	update = function(state: State, msg)
		if msg == "open" then
			return show(state, state.page)
		elseif msg == "close" then
			return state, { CLOSE_WINDOW }
		elseif msg == "toggle-alerts" then
			state.alerts = not state.alerts
			return state, { announce_alerts(state) }
		elseif msg == "toggle-elsewhere" then
			state.elsewhere = not state.elsewhere
			return state, { announce_alerts(state) }
		elseif msg == "copied" then
			state.copied = true
			return state
		elseif msg == "feedback" then
			-- Make it yours is the title bar's sparkles (Moonframe's feedback mod),
			-- in the main window.
			return state, { CLOSE_WINDOW, { to = "feedback", msg = "open" } }
		elseif msg == "check-updates" then
			-- Moonframe's updater: it says whether a newer release is out.
			return state, { CLOSE_WINDOW, { to = "app:command", msg = "check-for-updates" } }
		elseif msg == "releases" then
			state.error = nil
			return state, { { to = "provider:core", msg = { open = "releases" } } }
		elseif msg == "reset-layout" then
			state.layout = { tabs = DEFAULTS.tabs, headers = DEFAULTS.headers, status = DEFAULTS.status }
			return state, { announce(state.layout) }
		end
		if type(msg) ~= "table" then
			return
		end
		if msg.topic == "moonmux/layout" then
			-- What the topic keeps (saved, and the layout picker's before
			-- it moved here) is the choice.
			local value = msg.value or {}
			state.layout = { tabs = value.tabs or DEFAULTS.tabs, headers = value.headers or DEFAULTS.headers, status = value.status or DEFAULTS.status }
			return state
		elseif msg.topic == "moonmux/backend" then
			local choice = msg.value and msg.value.choice
			state.backend = if BACKENDS[choice] then choice else "auto"
			return state
		elseif msg.backend then
			if not BACKENDS[msg.backend] then
				return
			end
			state.backend = msg.backend
			return state, { announce_backend(msg.backend) }
		elseif msg.reply == "open" then
			state.error = if msg.ok then nil else msg.error
			return state
		elseif msg.open then
			return show(state, msg.open)
		elseif msg.select then
			-- The nav, inside the window.
			return select(state, msg.select)
		elseif msg.tabs or msg.headers or msg.status then
			state.layout.tabs = msg.tabs or state.layout.tabs
			state.layout.headers = msg.headers or state.layout.headers
			state.layout.status = msg.status or state.layout.status
			return state, { announce(state.layout) }
		end
	end,

	view = function(state: State, app)
		local core = app.core
		local function choice(title: string, description: string, event: string, selected: string, options: { any })
			return { title = title, description = description, control = segmented { event = event, selected = selected, options = options } }
		end

		local sections: { any }
		if state.page == "look" then
			local look = app.appearance
			sections = {
				{
					title = "Theme",
					rows = {
						{
							title = "Appearance",
							description = "Light or dark, or as your system is, switching when it does.",
							control = appearance { mode = look.mode },
						},
					},
				},
				{
					title = "Dark",
					description = "The theme for the window and its terminals while it is dark.",
					content = { theme_cards { themes = look.themes, dark = true, selected = look.dark_theme } },
				},
				{
					title = "Light",
					description = "The theme while it is light.",
					content = { theme_cards { themes = look.themes, dark = false, selected = look.light_theme } },
				},
				{
					title = "Layout",
					rows = {
						choice("Tabs", "How the tab strip in the title bar looks.", "tabs", state.layout.tabs, {
							{ id = "pills", label = "Pills" },
							{ id = "underline", label = "Underline" },
							{ id = "cards", label = "Cards" },
						}),
						choice("Pane headers", "A title row over each pane: its program, folder and buttons.", "headers", state.layout.headers, {
							{ id = "always", label = "Always" },
							{ id = "split", label = "When split" },
							{ id = "never", label = "Never" },
						}),
						choice("Status line", "herdr-core and agent counts under the window.", "status", state.layout.status, {
							{ id = "shown", label = "Shown" },
							{ id = "hidden", label = "Hidden" },
						}),
					},
				},
			}
		elseif state.page == "terminals" then
			local list = (core and core.backends and core.backends.list) or {}
			local auto = (core and core.backends and core.backends.auto) or ""
			local options = { { id = "auto", label = "Auto" } }
			local offered = { auto = true }
			for _, backend in list do
				if backend.installed then
					table.insert(options, { id = backend.id, label = backend.id })
					offered[backend.id] = true
				end
			end
			local notes = {}
			if not offered[state.backend] then
				table.insert(notes, ui.text { `{state.backend} is not installed here, so new panes run on Auto's choice.`, class = "st-note", wrap = true })
			end
			local rows: { any } = {}
			for _, backend in list do
				local detail = if not backend.installed then "Not installed"
					elseif not backend.ok then backend.error
					elseif backend.version ~= "" then `{backend.id} {backend.version}`
					else "Installed"
				table.insert(rows, ui.row {
					key = `b-{backend.id}`,
					class = "st-machine",
					align = "center",
					status_dot { status = if backend.installed and backend.ok then "done" else "idle", size = "small" },
					ui.column {
						grow = true,
						gap = "none",
						ui.text { if backend.id == auto then `{backend.id} · Auto's choice` else backend.id, weight = "medium" },
						ui.text { detail, class = "st-machine-detail", wrap = true },
					},
				})
			end
			if #list == 0 then
				table.insert(rows, ui.text { "Looking for herdr-core and tmux…", class = "st-note" })
			end
			sections = {
				{
					title = "New panes",
					description = "Panes keep running when Moonmux closes, in herdr-core or in tmux. Auto picks herdr-core when it is installed, else tmux.",
					rows = {
						{
							title = "Run new panes on",
							description = "Panes already open stay where they started, through relaunches too. To move one, close it and open a new one.",
							control = segmented { event = "backend", selected = state.backend, options = options },
						},
					},
					content = if #notes > 0 then notes else nil,
				},
				{
					title = "Backends",
					description = "The ones this build of Moonmux can use. Agents in both show in the sidebar: tmux's from this app's own server and your other tmux servers.",
					content = rows,
				},
			}
		elseif state.page == "machines" then
			local server = core and core.core
			local machines = core and core.machines
			local rows: { any } = {
				ui.row {
					class = "st-machine",
					align = "center",
					status_dot { status = if core and core.ok and server then "done" else "blocked", size = "small" },
					ui.column {
						grow = true,
						gap = "none",
						ui.text { "This computer", weight = "medium" },
						ui.text {
							if not core then "Looking for herdr-core…"
								elseif not core.ok or not server then "herdr-core is not reachable"
								elseif server and server.version ~= "" then `Connected · hc {server.version}`
								else "Connected",
							class = "st-machine-detail",
						},
					},
				},
			}
			if machines and not machines.ok then
				table.insert(rows, ui.text { machines.error, class = "st-error", wrap = true })
			elseif machines and #machines.list == 0 then
				table.insert(rows, ui.text { "No other machines yet.", class = "st-note" })
			elseif not machines then
				table.insert(rows, ui.text { "Reading machines…", class = "st-note" })
			end
			for _, machine in (machines and machines.list) or {} do
				local online = machine.state == "connected" or machine.state == "online"
				local state_text = if machine.state ~= "" then string.gsub(machine.state, "[_-]", " ") else "unknown"
				table.insert(rows, ui.row {
					key = `m-{machine.name}`,
					class = "st-machine",
					align = "center",
					status_dot { status = if online then "done" else "idle", size = "small" },
					ui.column {
						grow = true,
						gap = "none",
						ui.text { machine.name, weight = "medium", truncate = true },
						ui.text {
							`{machine.target} · {state_text} · {if machine.version ~= "" then "hc " .. machine.version else "hc version unknown"}`,
							class = "st-machine-detail",
							truncate = true,
						},
					},
				})
			end
			sections = {
				{
					title = "Machines",
					description = "Your computers, reached over ssh. Their terminals keep running when you disconnect.",
					content = rows,
				},
				{
					title = "Add a machine",
					description = "Run this in any terminal, with a host from your ssh config. herdr-core is installed there if it's missing, and the machine shows here.",
					content = {
						copy_field { text = ADD_MACHINE, msg = "copied", copied = state.copied },
					},
				},
			}
		elseif state.page == "alerts" then
			sections = {
				{
					title = "Sounds and notifications",
					rows = {
						{
							title = "When an agent needs you or finishes",
							description = "A sound and a notification when an agent in your spaces asks you something or finishes while you look elsewhere. Click the notification to go to it.",
							control = switch { on = state.alerts, msg = "toggle-alerts" },
						},
						{
							title = "Agents elsewhere too",
							description = "Also for agents started outside Moonmux or by another agent. A click peeks at them.",
							disabled = not state.alerts,
							control = switch { on = state.elsewhere, msg = "toggle-elsewhere", disabled = not state.alerts },
						},
					},
				},
			}
		elseif state.page == "updates" then
			local server = core and core.core
			sections = {
				{
					title = "Moonmux",
					rows = {
						{
							title = `Moonmux {(core and core.app_version) or ""}`,
							description = "Moonmux looks for a new version once a day and offers it when there is one.",
							control = ui.button { "Check Now", icon = "refresh-cw", small = true, variant = "outline", msg = "check-updates" },
						},
						{
							title = "Release notes",
							description = "What changed in each version, and every download.",
							control = ui.button { "Releases", icon = "external-link", small = true, variant = "outline", msg = "releases" },
						},
					},
					content = if state.error then { ui.text { state.error, class = "st-error", wrap = true } } else nil,
				},
				{
					title = "herdr-core",
					rows = {
						{
							title = if server and server.version ~= "" then `herdr-core {server.version}` else "herdr-core",
							description = if not core then "Looking for herdr-core…"
								elseif not server then "Not in use: herdr-core is not installed, or not part of this build."
								elseif not core.ok then core.error
								else "It runs your terminals, so they keep running through its updates and Moonmux's.",
						},
					},
				},
			}
		else
			sections = {
				{
					title = "Usage data",
					rows = {
						{
							title = "No telemetry",
							description = "Moonmux sends nothing about you or your use of it: no usage data, no crash reports, nothing from your terminals. It talks to herdr-core and tmux on this computer; herdr-core reaches your machines over ssh. Once a day it asks GitHub for its newest release, sending nothing but the request. Make it yours goes to the coding agent you pick, only when you send it.",
							control = ui.icon { name = "lock", tone = "success" },
						},
					},
				},
			}
		end

		return settings_sheet {
			title = TITLES[state.page],
			nav = {
				title = "Settings",
				selected = state.page,
				groups = { { items = PAGES } },
				footer = ui.row {
					class = "st-feedback",
					align = "center",
					msg = "feedback",
					ui.icon { name = "sparkles", size = "small" },
					ui.text { "Make it yours" },
				},
			},
			sections = sections,
			status = "Changes apply at once.",
			buttons = if state.page == "look"
				then { { label = "Reset layout", msg = "reset-layout" }, DONE }
				else { DONE },
		}
	end,
}
