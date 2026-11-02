-- Appearance: System, Light or Dark as segments. Moonframe lets a control
-- flip light and dark (the `toggle-theme` command, which also goes back to
-- the stock theme) but not set either, nor follow the system after launch,
-- so Light and Dark each flip to themselves and System is shown disabled.
-- Settings > Look and the Welcome's first page draw it.

return component {
	title = "Appearance",
	category = "Controls",
	icon = "sun",
	description = "System, Light or Dark as segments; Light and Dark switch the window at once.",
	when = "Choosing light or dark for the whole window, in Settings or a welcome. For any other one-of-many choice use `segmented`.",
	props = {
		["dark"] = "whether the window is dark now (`app.dark`)",
	},
	styles = {
		[".appearance"] = { background = "muted", border = "all", border_color = "border", radius = 9, padding = 2, gap = 2 },
		[".appearance-option"] = { padding = { 4, 12 }, gap = 6, radius = 7, color = "muted-foreground" },
		[".appearance-option:hover"] = { color = "foreground" },
		[".appearance-option:selected"] = { background = "background", color = "foreground", shadow = "0 1 2 #00000024" },
		[".appearance-off"] = { opacity = 0.45 },
	},
	example = { dark = false },

	build = function(props)
		local dark = props.dark == true
		local function option(label: string, icon: string, chosen: boolean)
			return ui.row {
				class = "appearance-option",
				align = "center",
				selected = chosen,
				-- The chosen one does nothing; the other flips to itself.
				command = if chosen then nil else "toggle-theme",
				ui.icon { name = icon :: any, size = "small" },
				ui.text { label },
			}
		end
		return ui.row {
			class = "appearance",
			hug = true,
			align = "center",
			ui.row {
				class = "appearance-option appearance-off",
				align = "center",
				tooltip = "Moonframe follows the system only as Moonmux starts",
				ui.icon { name = "monitor", size = "small" },
				ui.text { "System" },
			},
			option("Light", "sun", not dark),
			option("Dark", "moon", dark),
		}
	end,
}
