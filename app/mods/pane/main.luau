-- One pane: a live shell kept by herdr-core, mounted by the workspace as an
-- instance per pane (`ui.mount { "pane", id = "p3", props = { cwd } }`).
-- Being its own instance, each pane owns its terminal (`pane#p3/term`),
-- which is how `app.core` knows which pane an agent runs in.

return mod {
	title = "Pane",

	init = function(previous)
		return previous or {}
	end,

	view = function(_state, _app, props)
		local cwd = if props and props.cwd and props.cwd ~= "" then props.cwd else "~"
		return ui.column {
			grow = true,
			gap = "none",
			ui.terminal { id = "term", cwd = cwd, backend = "herdr-core", class = "pane-terminal" },
		}
	end,
}
