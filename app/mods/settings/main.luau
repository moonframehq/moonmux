-- Settings: a dialog over the window with its pages in a nav on the left
-- (Look, Machines, Alerts, Updates, Privacy) and Send feedback under them.
-- ⌘, (File > Settings…), the sidebar's Settings row and the title bar's
-- layout button open it; `{ open = "machines" }` opens it on a page.
--
-- Look holds the layout choices (tab style, pane headers, status line),
-- kept on the topic `herdr/layout` as { tabs, headers, status }, which the
-- workspace and the shell draw to match, and the appearance (System, Light
-- or Dark) and a light and a dark theme, which Moonframe keeps
-- (`app.appearance`). Alerts are shared on `herdr/alerts` as
-- { enabled, elsewhere } for mods/alerts. Machines lists `app.core.machines`
-- (`hc machine ls`), read-only.

local settings_page = use "settings-page"
local segmented = use "segmented"
local switch = use "switch"
local status_dot = use "status-dot"
local copy_field = use "copy-field"
local appearance = use "appearance"
local theme_cards = use "theme-cards"

type Layout = { tabs: string, headers: string, status: string }
type State = {
	open: boolean,
	page: string,
	-- Sounds and notifications when an agent needs you or finishes (on by
	-- default), and for agents elsewhere too (off by default).
	alerts: boolean,
	elsewhere: boolean,
	layout: Layout,
	copied: boolean,
	-- Why the releases page did not open, from the core provider's reply.
	error: string?,
}

local DEFAULTS: Layout = { tabs = "pills", headers = "always", status = "shown" }
local PAGES = {
	{ id = "look", label = "Look", icon = "palette" },
	{ id = "machines", label = "Machines", icon = "server" },
	{ id = "alerts", label = "Alerts", icon = "bell" },
	{ id = "updates", label = "Updates", icon = "refresh-cw" },
	{ id = "privacy", label = "Privacy", icon = "shield" },
}
local TITLES = { look = "Look", machines = "Machines", alerts = "Alerts", updates = "Updates", privacy = "Privacy" }
local ADD_MACHINE = "hc machine add <host>"
local DONE = { label = "Done", msg = "close", variant = "primary" }

local function announce(layout: Layout)
	return { to = "topic:herdr/layout", msg = { tabs = layout.tabs, headers = layout.headers, status = layout.status } }
end

local function announce_alerts(state: State)
	return { to = "topic:herdr/alerts", msg = { enabled = state.alerts, elsewhere = state.elsewhere } }
end

-- Opens on `page`, reading the machines again for their page.
local function show(state: State, page: string)
	state.open = true
	state.page = if TITLES[page] then page else "look"
	state.copied = false
	state.error = nil
	if state.page == "machines" then
		return state, { { to = "provider:core", msg = { refresh = "machines" } } }
	end
	return state, {}
end

return mod {
	title = "Settings",
	subscribe = { "herdr/layout" },

	styles = {
		[".st-dialog"] = { padding = 0, radius = 12, clip = true, shadow = "large" },
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
			open = false,
			page = previous and previous.page or "look",
			alerts = if previous then previous.alerts ~= false else true,
			elsewhere = previous ~= nil and previous.elsewhere == true,
			layout = { tabs = layout.tabs or DEFAULTS.tabs, headers = layout.headers or DEFAULTS.headers, status = layout.status or DEFAULTS.status },
			copied = false,
			error = nil,
		}
		return state, { announce_alerts(state) }
	end,

	update = function(state: State, msg)
		if msg == "open" then
			return show(state, state.page)
		elseif msg == "close" then
			state.open = false
			return state
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
			-- Send feedback is the title bar's `?` (Moonframe's feedback mod).
			state.open = false
			return state, { { to = "feedback", msg = "open" } }
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
		if msg.topic == "herdr/layout" then
			-- What the topic keeps (saved, and the layout picker's before
			-- it moved here) is the choice.
			local value = msg.value or {}
			state.layout = { tabs = value.tabs or DEFAULTS.tabs, headers = value.headers or DEFAULTS.headers, status = value.status or DEFAULTS.status }
			return state
		elseif msg.reply == "open" then
			state.error = if msg.ok then nil else msg.error
			return state
		elseif msg.open then
			return show(state, msg.open)
		elseif msg.select then
			return show(state, msg.select)
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
		elseif state.page == "machines" then
			local server = core and core.core
			local machines = core and core.machines
			local rows: { any } = {
				ui.row {
					class = "st-machine",
					align = "center",
					status_dot { status = if core and core.ok then "done" else "blocked", size = "small" },
					ui.column {
						grow = true,
						gap = "none",
						ui.text { "This computer", weight = "medium" },
						ui.text {
							if not core then "Looking for herdr-core…"
								elseif not core.ok then "herdr-core is not reachable"
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
							description = "Also for agents started outside Herdr or by another agent. A click peeks at them.",
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
					title = "Herdr",
					rows = {
						{
							title = `Herdr {(core and core.app_version) or ""}`,
							description = "New versions are on the releases page. Herdr doesn't look for them itself.",
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
								elseif not core.ok then core.error
								else "It runs your terminals, so they keep running through its updates and Herdr's.",
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
							description = "Herdr sends nothing about you or your use of it: no usage data, no crash reports, nothing from your terminals. It talks to herdr-core on this computer, which reaches your machines over ssh. Send feedback goes to the coding agent you pick, only when you send it.",
							control = ui.icon { name = "lock", tone = "success" },
						},
					},
				},
			}
		end

		return ui.overlay {
			open = state.open,
			dismiss = "close",
			backdrop = true,
			width = 860,
			class = "st-dialog",
			content = settings_page {
				height = 600,
				title = TITLES[state.page],
				nav = {
					title = "Settings",
					selected = state.page,
					groups = { { items = PAGES } },
					footer = ui.row {
						class = "st-feedback",
						align = "center",
						msg = "feedback",
						ui.icon { name = "message-square", size = "small" },
						ui.text { "Send feedback" },
					},
				},
				sections = sections,
				status = "Changes apply at once.",
				buttons = if state.page == "look"
					then { { label = "Reset layout", msg = "reset-layout", variant = "ghost" }, DONE }
					else { DONE },
			},
		}
	end,
}
