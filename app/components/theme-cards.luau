-- Theme cards: the light or the dark themes Moonframe lists
-- (`app.appearance.themes`) as herdr-app's picker draws them: a small
-- swatch in the theme's background with a line in its foreground and dots
-- of its accent and colors, the name under it, and a check on the chosen
-- one. A click sends `{ to = "app:theme", msg = { dark = name } }` (or
-- `light`). The cards wrap, in a box that scrolls past a few rows, so a
-- long list stays one short section.

return component {
	title = "Theme cards",
	category = "Forms",
	icon = "palette",
	description = "The light or the dark themes as small swatch cards; a click picks one for that appearance.",
	when = "Picking the theme light (or dark) windows show, from `app.appearance.themes`. For one colour use `swatches`; for light or dark itself use `appearance`.",
	props = {
		["themes"] = "`app.appearance.themes`: { { name, dark, background, foreground, accent, colors } }",
		["dark"] = "true for the dark themes, false for the light ones",
		["selected"] = "the chosen theme's name (`app.appearance.dark_theme` or `light_theme`)",
	},
	styles = {
		[".theme-cards"] = { gap = 8 },
		[".theme-card"] = { width = 124, gap = 6, padding = 6, radius = 10, border = "all", border_color = "#00000000" },
		[".theme-card:hover"] = { background = "list-hover" },
		[".theme-card:selected"] = { border_color = "accent" },
		[".theme-card-swatch"] = { height = 46, padding = 7, gap = 6, radius = 6, border = "all", border_color = "border" },
		[".theme-card-line"] = { width = 44, height = 4, radius = 2 },
		[".theme-card-dots"] = { gap = 4 },
		[".theme-card-label"] = { gap = 5 },
		[".theme-card-dot"] = { width = 8, height = 8, radius = 4 },
		[".theme-card-name"] = { size = "small", color = "muted-foreground" },
		[".theme-card-chosen"] = { color = "foreground" },
	},
	example = {
		dark = true,
		selected = "Nord",
		themes = {
			{ name = "Default", dark = true, background = "#1e1e1e", foreground = "#d4d4d4", accent = "#3b82f6", colors = { "#f87171", "#4ade80", "#facc15", "#60a5fa", "#c084fc", "#22d3ee" } },
			{ name = "Nord", dark = true, background = "#2e3440", foreground = "#d8dee9", accent = "#88c0d0", colors = { "#bf616a", "#a3be8c", "#ebcb8b", "#81a1c1", "#b48ead", "#88c0d0" } },
		},
	},

	build = function(props)
		local dark = props.dark == true
		local key = if dark then "dark" else "light"
		local cards = {}
		for index, theme: any in (props.themes or {}) :: { any } do
			if (theme.dark == true) ~= dark then
				continue
			end
			local chosen = theme.name == props.selected
			local colors = theme.colors or {}
			local dots = {}
			-- The accent, then red, green, yellow and magenta, as herdr-app.
			for _, color in { theme.accent, colors[1], colors[2], colors[3], colors[5] } do
				if color then
					table.insert(dots, ui.row { class = "theme-card-dot", style = { background = color } })
				end
			end
			table.insert(cards, ui.column {
				-- Keys are a-z A-Z 0-9 - _; names have spaces and more.
				key = `{key}-{index}-{string.sub((string.gsub(theme.name, "[^%w_-]", "-")), 1, 40)}`,
				class = "theme-card",
				selected = chosen,
				tooltip = theme.name,
				to = if chosen then nil else "app:theme",
				msg = if chosen then nil else { [key] = theme.name },
				ui.column {
					class = "theme-card-swatch",
					style = { background = theme.background },
					ui.row { class = "theme-card-line", style = { background = theme.foreground } },
					ui.row { class = "theme-card-dots", table.unpack(dots) },
				},
				ui.row {
					class = "theme-card-label",
					align = "center",
					ui.text { theme.name, class = if chosen then "theme-card-name theme-card-chosen" else "theme-card-name", truncate = true },
					chosen and ui.icon { name = "check", size = "small", tone = "accent" },
				},
			})
		end
		return ui.column {
			max_height = 240,
			scroll = "y",
			ui.row { class = "theme-cards", wrap = true, table.unpack(cards) },
		}
	end,
}
