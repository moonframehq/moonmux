-- The layout picker: how the workspace draws its tabs and panes, over
-- style.luau. It draws in no region: the title bar's layout button opens it
-- in a window of its own (`window = "layout"`), which stays open while you
-- try one choice after another. Choices are kept across launches and
-- published on the topic `herdr/layout` as { tabs, headers, status }; the
-- workspace and the shell draw to match.

local segmented = use "segmented"

type State = {
	tabs: string,
	headers: string,
	status: string,
}

local DEFAULTS: State = { tabs = "pills", headers = "split", status = "shown" }

local function announce(state: State)
	return { to = "topic:herdr/layout", msg = { tabs = state.tabs, headers = state.headers, status = state.status } }
end

return mod {
	title = "Layout",

	styles = {
		[".lp"] = { padding = 18, gap = 16 },
		[".lp-label"] = { weight = "medium" },
		[".lp-detail"] = { size = "small", color = "muted-foreground" },
	},

	init = function(previous: State?): State
		local state: State = {
			tabs = previous and previous.tabs or DEFAULTS.tabs,
			headers = previous and previous.headers or DEFAULTS.headers,
			status = previous and previous.status or DEFAULTS.status,
		}
		return state, { announce(state) }
	end,

	update = function(state: State, msg)
		if type(msg) ~= "table" then
			if msg == "reset" then
				state.tabs, state.headers, state.status = DEFAULTS.tabs, DEFAULTS.headers, DEFAULTS.status
				return state, { announce(state) }
			end
			return
		end
		if msg.tabs then
			state.tabs = msg.tabs
		elseif msg.headers then
			state.headers = msg.headers
		elseif msg.status then
			state.status = msg.status
		else
			return
		end
		return state, { announce(state) }
	end,

	view = function(state: State)
		local function choice(label: string, detail: string, event: string, selected: string, options: { any })
			return ui.column {
				gap = "small",
				ui.text { label, class = "lp-label" },
				ui.text { detail, class = "lp-detail" },
				ui.row { segmented { grow = true, event = event, selected = selected, options = options } },
			}
		end
		return ui.column {
			class = "lp",
			grow = true,
			choice("Tabs", "How the tab strip in the title bar looks.", "tabs", state.tabs, {
				{ id = "pills", label = "Pills" },
				{ id = "underline", label = "Underline" },
				{ id = "cards", label = "Cards" },
			}),
			choice("Pane headers", "A title row over each pane: its agent, address and buttons.", "headers", state.headers, {
				{ id = "split", label = "When split" },
				{ id = "always", label = "Always" },
				{ id = "never", label = "Never" },
			}),
			choice("Status line", "herdr-core and agent counts under the window.", "status", state.status, {
				{ id = "shown", label = "Shown" },
				{ id = "hidden", label = "Hidden" },
			}),
			ui.spacer(),
			ui.row { justify = "end", ui.button { "Reset", variant = "ghost", msg = "reset" } },
		}
	end,
}
