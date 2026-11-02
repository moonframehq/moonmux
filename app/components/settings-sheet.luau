-- The Settings window's body: Moonframe's `settings-page` laid out the same
-- way (`settings-nav` on the left, a scrolling page of titled sections of
-- `settings-row`s), with a footer whose buttons are full size. Moonframe's
-- component always draws its footer buttons small, and a button's size is
-- not something a stylesheet can change, so the app draws its own.

local settings_nav = use "settings-nav"
local settings_row = use "settings-row"

type Row = {
	title: string,
	description: string?,
	disabled: boolean?,
	-- The control: one element, or a list of them.
	control: any,
}

type Section = {
	title: string?,
	description: string?,
	rows: { Row }?,
	-- Elements instead of (or after) rows.
	content: { any }?,
}

type Button = { label: string, msg: any, variant: string? }

type Props = {
	nav: any,
	title: string?,
	sections: { Section }?,
	status: string?,
	buttons: { Button }?,
	height: number?,
}

return component {
	title = "Settings sheet",
	category = "Layout",
	icon = "settings",
	description = "Settings' body: its nav, a scrolling page of titled sections and rows, and a footer with a status and full-size buttons.",
	when = "The Settings window. Anywhere else use Moonframe's `settings-page`.",
	props = {
		["nav"] = "props for `settings-nav`: { groups, selected, title, footer }",
		["title"] = "the page's title",
		["sections"] = "{ { title, description, rows = { { title, description, disabled, control } }, content } }",
		["status"] = "the footer's text",
		["buttons"] = "the footer's buttons { { label, msg, variant } }, outline unless a variant is given",
		["height"] = "the body's height in pixels; it grows to fill without one",
	},
	styles = {
		[".settings-sheet"] = { background = "background", border = "all", border_color = "border", radius = 10, gap = 0 },
		[".settings-sheet-main"] = { gap = 0 },
		[".settings-sheet-scroll"] = { padding = { 24, 28 }, gap = 22 },
		[".settings-sheet-title"] = { size = "title", weight = "bold" },
		[".settings-sheet-section"] = { gap = 0 },
		[".settings-sheet-heading"] = {
			case = "upper",
			size = "small",
			weight = "medium",
			color = "muted-foreground",
			padding = { 0, 0, 6, 0 },
			border = "bottom",
			border_color = "border",
		},
		[".settings-sheet-section-description"] = { size = "small", color = "muted-foreground", padding = { 6, 0, 0, 0 } },
		[".settings-sheet-footer"] = { padding = { 12, 16 }, gap = 10, border = "top", border_color = "border", background = "sidebar" },
		[".settings-sheet-status"] = { size = "small", color = "muted-foreground" },
		[".settings-sheet-dot"] = { radius = 4, background = "success" },
	},
	example = {
		height = 360,
		nav = { title = "Settings", selected = "look", groups = { { items = { { id = "look", label = "Look", icon = "palette" } } } } },
		title = "Look",
		sections = { { title = "Layout", rows = { { title = "Status line", description = "Counts under the window." } } } },
		status = "Changes apply at once.",
		buttons = { { label = "Done", msg = "close", variant = "primary" } },
	},

	build = function(props: Props)
		local page: { any } = {}
		if props.title then
			page[#page + 1] = ui.text { props.title, class = "settings-sheet-title" }
		end
		for _, section in ipairs(props.sections or {}) do
			local children: { any } = {}
			if section.title then
				children[#children + 1] = ui.text { section.title, class = "settings-sheet-heading" }
			end
			if section.description then
				children[#children + 1] = ui.text { section.description, class = "settings-sheet-section-description" }
			end
			for _, row in ipairs(section.rows or {}) do
				local controls: { any } = {}
				if type(row.control) == "table" and row.control.type == nil then
					controls = row.control
				elseif row.control ~= nil then
					controls = { row.control }
				end
				children[#children + 1] = settings_row {
					title = row.title,
					description = row.description,
					disabled = row.disabled,
					table.unpack(controls),
				}
			end
			for _, element in ipairs(section.content or {}) do
				children[#children + 1] = element
			end
			page[#page + 1] = ui.column { class = "settings-sheet-section", shrink = false, table.unpack(children) }
		end

		local buttons: { any } = {}
		for _, button in ipairs(props.buttons or {}) do
			buttons[#buttons + 1] = ui.button {
				button.label,
				variant = (button.variant or "outline") :: any,
				msg = button.msg,
			}
		end

		local nav = {}
		for key, value in props.nav or {} do
			nav[key] = value
		end
		nav.scroll = true

		return ui.column {
			class = "settings-sheet",
			height = props.height,
			grow = props.height == nil,
			ui.row {
				grow = true,
				gap = "none",
				align = "stretch",
				settings_nav(nav),
				ui.column {
					class = "settings-sheet-main",
					grow = true,
					ui.column { class = "settings-sheet-scroll", grow = true, scroll = true, table.unpack(page) },
				},
			},
			ui.row {
				class = "settings-sheet-footer",
				align = "center",
				ui.row { class = "settings-sheet-dot", width = 8, height = 8 },
				ui.text { props.status or "", class = "settings-sheet-status" },
				ui.spacer(),
				table.unpack(buttons),
			},
		}
	end,
}
