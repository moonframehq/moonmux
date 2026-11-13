-- The status line under the window: the backends in use (herdr-core, tmux)
-- with their versions and how many terminals they keep, then how many
-- agents work, wait on you, or finished, and at the right "⌘⇧P commands",
-- which opens the palette.

local status_dot = use "status-dot"
local kbd = use "kbd"

return mod {
	title = "Status",

	styles = {
		[".st"] = { padding = { 0, 14 }, gap = 14 },
		[".st-text"] = { size = "small", color = "muted-foreground" },
		[".st-item"] = { gap = 6 },
		[".st-commands"] = { padding = { 1, 6 }, radius = 6, gap = 6 },
		[".st-commands:hover"] = { background = "list-hover" },
	},

	view = function(_state, app)
		local core = app.core
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
