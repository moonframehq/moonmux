-- One pane: a live shell kept by herdr-core or tmux, mounted by the
-- workspace as an instance per pane (`ui.mount { "pane", id = "p3", props =
-- { cwd, backend } }`). Being its own instance, each pane owns its terminal
-- (`pane#p3/term`: herdr-core labels it so, tmux names its session
-- `pane_p3+term`), which is how `app.core` knows which pane an agent runs
-- in. Moonframe opens the terminal on `backend` and keeps it there: a
-- change to the prop once it is open applies only after it ends.

return mod {
	title = "Pane",

	init = function(previous)
		return previous or {}
	end,

	view = function(_state, _app, props)
		local cwd = if props and props.cwd and props.cwd ~= "" then props.cwd else "~"
		local backend: any = if props and (props.backend == "herdr-core" or props.backend == "tmux") then props.backend else "auto"
		return ui.column {
			grow = true,
			gap = "none",
			ui.terminal { id = "term", cwd = cwd, backend = backend, class = "pane-terminal" },
		}
	end,
}
