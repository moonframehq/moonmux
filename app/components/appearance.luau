-- Appearance: System, Light or Dark as segments, sent to the app
-- (`app:appearance`), which keeps the choice across launches and, on
-- System, follows the system as it switches. Settings > Look and the
-- Welcome's first page draw it.

return component {
	title = "Appearance",
	category = "Controls",
	icon = "sun",
	description = "System, Light or Dark as segments; a click switches the window at once and is kept.",
	when = "Choosing light or dark for the whole window, in Settings or a welcome. For any other one-of-many choice use `segmented`.",
	props = {
		["mode"] = "what the user chose: `system`, `light` or `dark` (`app.appearance.mode`)",
	},
	styles = {
		[".appearance"] = { background = "muted", border = "all", border_color = "border", radius = 9, padding = 2, gap = 2 },
		[".appearance-option"] = { padding = { 4, 12 }, gap = 6, radius = 7, color = "muted-foreground" },
		[".appearance-option:hover"] = { color = "foreground" },
		[".appearance-option:selected"] = { background = "background", color = "foreground", shadow = "0 1 2 #00000024" },
	},
	example = { mode = "system" },

	build = function(props)
		local mode = props.mode or "system"
		local function option(id: string, label: string, icon: string)
			local chosen = mode == id
			return ui.row {
				class = "appearance-option",
				align = "center",
				selected = chosen,
				-- The chosen one does nothing.
				to = if chosen then nil else "app:appearance",
				msg = if chosen then nil else id,
				ui.icon { name = icon :: any, size = "small" },
				ui.text { label },
			}
		end
		return ui.row {
			class = "appearance",
			hug = true,
			align = "center",
			option("system", "System", "monitor"),
			option("light", "Light", "sun"),
			option("dark", "Dark", "moon"),
		}
	end,
}
