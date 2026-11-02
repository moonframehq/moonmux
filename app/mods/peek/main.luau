-- Peek: a sheet over the window showing another terminal's screen, from
-- herdr-core (`hc read`) or tmux (`capture-pane`), through
-- `provider:core`'s `watch`, with a line to send it text and buttons for
-- the keys an agent asks for (Enter, Esc, an interrupt). Opened with
-- `{ open = "t7" }` (or a tmux pane, `default:%12`) from the sidebar.

local icon_button = use "icon-button"
local status_dot = use "status-dot"

type State = { address: string?, text: string, error: string? }

local LABELS = { working = "Working", blocked = "Needs you", done = "Done", idle = "Idle" }

return mod {
	title = "Peek",

	styles = {
		[".peek"] = { background = "card", radius = { 14, 0, 0, 14 }, shadow = "large", transition = "slide-fade normal ease-out" },
		[".peek-head"] = { padding = { 12, 12, 12, 16 }, gap = 10, border = "bottom", border_color = "border" },
		[".peek-screen"] = { padding = { 10, 14 }, background = "background" },
		[".peek-text"] = { font = "mono", size = "small" },
		[".peek-foot"] = { padding = { 10, 12 }, gap = 8, border = "top", border_color = "border" },
		[".peek-error"] = { size = "small", color = "danger" },
	},

	init = function(): State
		return { address = nil, text = "", error = nil }
	end,

	update = function(state: State, msg)
		if type(msg) == "table" and msg.open then
			state.address = msg.open
			state.text = ""
			state.error = nil
			return state, { { to = "provider:core", msg = { watch = msg.open } } }
		elseif msg == "close" then
			state.address = nil
			return state, { { to = "provider:core", msg = { watch = "" } } }
		elseif type(msg) == "table" and msg.msg == "text" then
			state.text = msg.value
			return state
		elseif msg == "send" and state.address and state.text ~= "" then
			local text = state.text
			state.text = ""
			return state, { { to = "provider:core", msg = { send = state.address, text = text } } }
		elseif type(msg) == "table" and msg.key and state.address then
			return state, { { to = "provider:core", msg = { keys = state.address, key = msg.key } } }
		elseif msg == "interrupt" and state.address then
			return state, { { to = "provider:core", msg = { interrupt = state.address } } }
		elseif type(msg) == "table" and msg.reply then
			state.error = if msg.ok then nil else msg.error
			return state
		end
	end,

	view = function(state: State, app)
		local core = app.core
		local terminal = nil
		for _, each in (core and core.terminals) or {} do
			if each.address == state.address then
				terminal = each
			end
		end
		local peek = core and core.peek
		local screen = if peek and peek.address == state.address then peek else nil
		local status = if terminal and terminal.status ~= "" then terminal.status else "idle"

		return ui.overlay {
			open = state.address ~= nil,
			dismiss = "close",
			place = "right",
			width = 760,
			backdrop = true,
			class = "peek",
			keys = { ["secondary-enter"] = "send" :: any },
			content = ui.column {
				grow = true,
				gap = "none",
				ui.row {
					class = "peek-head",
					align = "center",
					ui.icon { name = "eye", tone = "muted" },
					ui.column {
						grow = true,
						gap = "none",
						ui.text { if terminal then `{terminal.agent} · {terminal.backend} {terminal.target}` else (state.address or ""), weight = "semibold", truncate = true },
						ui.text { if terminal then terminal.cwd else "This terminal has ended", tone = "muted", size = "small", truncate = true },
					},
					terminal and status_dot { status = status, chip = true },
					icon_button { icon = "close", tooltip = "Close (Esc)", msg = "close" },
				},
				ui.column {
					class = "peek-screen",
					grow = true,
					scroll = "both",
					if screen and screen.error ~= ""
						then ui.text { screen.error, class = "peek-error", wrap = true }
						else ui.text { if screen then screen.text else "Reading…", class = "peek-text", wrap = false },
				},
				ui.column {
					class = "peek-foot",
					shrink = false,
					ui.row {
						align = "center",
						ui.input {
							id = "peek-text",
							value = state.text,
							placeholder = if status == "blocked" then "Answer it…" else "Send it a message…",
							change = "text",
							submit = "send",
							grow = true,
							focus = state.address ~= nil,
						},
						ui.button { "Send", variant = "primary", msg = "send", disabled = state.text == "" },
					},
					ui.row {
						align = "center",
						gap = "small",
						ui.button { "Enter", small = true, variant = "outline", msg = { key = "enter" } },
						ui.button { "Esc", small = true, variant = "outline", msg = { key = "esc" } },
						ui.button { "↑", small = true, variant = "outline", msg = { key = "up" } },
						ui.button { "↓", small = true, variant = "outline", msg = { key = "down" } },
						ui.button { "Interrupt", small = true, variant = "outline", msg = "interrupt", tooltip = "Press its stop key" },
						ui.spacer(),
						state.error and ui.text { state.error, class = "peek-error", truncate = true },
						terminal and ui.text { LABELS[status] or "", tone = "muted", size = "small" },
					},
				},
			},
		}
	end,
}
