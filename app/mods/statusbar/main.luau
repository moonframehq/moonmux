-- The status line under the window: herdr-core's name and version and how
-- many terminals it keeps, then how many agents work, wait on you, or
-- finished.

local status_dot = use "status-dot"

return mod {
	title = "Status",

	styles = {
		[".st"] = { padding = { 0, 14 }, gap = 14 },
		[".st-text"] = { size = "small", color = "muted-foreground" },
		[".st-item"] = { gap = 6 },
	},

	view = function(_state, app)
		local core = app.core
		local counts = core and core.counts or { live = 0, working = 0, blocked = 0, done = 0 }
		local server = core and core.core

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

		local left = if not core
			then ui.text { "Looking for herdr-core…", class = "st-text" }
			elseif not core.ok then ui.text { "herdr-core is not reachable", class = "st-text", tone = "danger" }
			else ui.row {
				class = "st-item",
				align = "center",
				ui.icon { name = "server", tone = "muted", size = "small" },
				ui.text {
					if server and server.version ~= "" then `herdr-core {server.version} on {server.name}` else "herdr-core",
					class = "st-text",
				},
				ui.text { `· {counts.live} terminal{if counts.live == 1 then "" else "s"}`, class = "st-text" },
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
		}
	end,
}
