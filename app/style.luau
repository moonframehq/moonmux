-- Moonmux's look: a soft window behind one card of terminals, quiet
-- controls, round corners. Light first, with dark colours for each.
return style {
	font_size = 13,
	radius = 8,

	colors = {
		background = { light = "#eceef1", dark = "#141519" },
		sidebar = { light = "#eceef1", dark = "#141519" },
		card = { light = "#ffffff", dark = "#1d1f24" },
		border = { light = "#dfe2e7", dark = "#2a2d34" },
		["list-hover"] = { light = "#0000000a", dark = "#ffffff0d" },
		["muted-foreground"] = { light = "#6f7580", dark = "#8b919c" },
		accent = { light = "#4f6bed", dark = "#8aa0ff" },
	},

	rules = {
		pane = { transition = "slide-fade normal ease-out" },
		-- The sidebar fades rather than slides: shrinking its width as it
		-- folds would rewrap its text on every frame.
		[".side"] = { background = "background", transition = "fade fast ease-out" },
		[".middle"] = { background = "background", padding = { 0, 10, 0, 0 } },
		terminal = { background = "card", padding = { 6, 8 } },
		overlay = { transition = "slide-fade fast ease-out" },
		["overlay-backdrop"] = { background = "#000000", opacity = 0.3 },
	},
}
