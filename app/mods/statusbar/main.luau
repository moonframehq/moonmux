-- The status line under the window: the backends in use (herdr-core, tmux)
-- with their versions and how many terminals they keep, then how many
-- agents work, wait on you, or finished, and at the right "⌘⇧P commands",
-- which opens the palette. Beside it, this machine's CPU and memory, each
-- a sparkline of the last samples with its current percentage.

local status_dot = use "status-dot"
local kbd = use "kbd"
local sparkline = use "sparkline"

type State = { cpu: { number }, memory: { number } }

-- Samples kept for each sparkline, one a second.
local HISTORY = 16

local function percent(fraction: number): string
	return `{math.floor(math.clamp(fraction, 0, 1) * 100 + 0.5)}%`
end

local function push(history: { number }, value: number)
	table.insert(history, math.clamp(value, 0, 1))
	while #history > HISTORY do
		table.remove(history, 1)
	end
end

return mod {
	title = "Status",
	subscribe = { "provider/system" },

	init = function(previous: State?): State
		return { cpu = if previous then previous.cpu or {} else {}, memory = if previous then previous.memory or {} else {} }
	end,

	update = function(state: State, msg: any): State
		if type(msg) == "table" and msg.topic == "provider/system" and type(msg.value) == "table" then
			local system = msg.value
			if type(system.cpu) == "number" then
				push(state.cpu, system.cpu)
			end
			if type(system.memory) == "number" then
				push(state.memory, system.memory)
			end
		end
		return state
	end,

	styles = {
		[".st"] = { padding = { 0, 14 }, gap = 14 },
		[".st-text"] = { size = "small", color = "muted-foreground" },
		[".st-item"] = { gap = 6 },
		[".st-commands"] = { padding = { 1, 6 }, radius = 6, gap = 6 },
		[".st-commands:hover"] = { background = "list-hover" },
		[".st-load"] = { gap = 5 },
		[".st-load-value"] = { size = "small", color = "muted-foreground", font = "mono" },
	},

	view = function(state: State, app)
		local core = app.core

		-- One reading: its icon, the sparkline, and the latest value in a
		-- fixed-width box so a changing number never shifts its neighbours.
		local function load(icon: string, label: string, history: { number }, detail: string?): any
			local latest = history[#history]
			if not latest then
				return nil
			end
			return ui.row {
				class = "st-load",
				align = "center",
				tooltip = if detail then `{label} {percent(latest)} · {detail}` else `{label} {percent(latest)}`,
				ui.icon { name = icon :: any, tone = "muted", size = "small" },
				sparkline { values = history, samples = HISTORY, hot = 0.85 },
				ui.row { width = 32, justify = "end", ui.text { percent(latest), class = "st-load-value" } },
			}
		end

		local system = app.system
		local memory_detail = nil
		if system and type(system.memory_used) == "number" and type(system.memory_total) == "number" then
			local gib = 1024 * 1024 * 1024
			memory_detail = string.format("%.1f of %.1f GB", system.memory_used / gib, system.memory_total / gib)
		end
		local counts = core and core.counts or { live = 0, working = 0, blocked = 0, done = 0 }

		local function count(status: string, label: string, n: number): any
			if n == 0 then
				return nil
			end
			return ui.row {
				class = "st-item",
				align = "center",
				status_dot { status = status, size = "small" },
				ui.text { `{n} {label}`, class = "st-text" },
			}
		end

		-- Each installed backend that answers, with its version.
		local names = {}
		for _, backend in (core and core.backends and core.backends.list) or {} do
			if backend.installed and backend.ok then
				table.insert(names, if backend.version ~= "" then `{backend.id} {backend.version}` else backend.id)
			end
		end
		local left = if not core
			then ui.text { "Looking for herdr-core and tmux…", class = "st-text" }
			elseif not core.ok then ui.text { core.error, class = "st-text", tone = "danger", truncate = true }
			else ui.row {
				class = "st-item",
				align = "center",
				ui.icon { name = "server", tone = "muted", size = "small" },
				ui.text { table.concat(names, " · "), class = "st-text" },
				ui.text { `· {counts.live} terminal{if counts.live == 1 then "" else "s"}`, class = "st-text" },
				core.error ~= "" and ui.icon { name = "triangle-alert", tone = "danger", size = "small", tooltip = core.error },
			}

		return ui.row {
			class = "st",
			height = 26,
			shrink = false,
			align = "center",
			left,
			ui.spacer(),
			count("blocked", "need you", counts.blocked),
			count("working", "working", counts.working),
			count("done", "done", counts.done),
			load("cpu", "CPU", state.cpu),
			load("memory-stick", "Memory", state.memory, memory_detail),
			ui.row {
				class = "st-commands",
				align = "center",
				msg = "open",
				to = "palette",
				tooltip = "Every action, by name",
				kbd { keys = "secondary-shift-p", small = true, plain = true },
				ui.text { "commands", class = "st-text" },
			},
		}
	end,
}
