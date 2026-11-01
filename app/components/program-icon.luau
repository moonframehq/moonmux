-- A program's icon, by the key `app.core` gives it (`program_key` on a
-- terminal, `key` in `programs`): Moonframe's agent logos, a terminal for a
-- shell, the app's own Pi mark (app/icons, see NOTICE), a bot for an agent
-- the app does not know and a terminal for any other program.

type Props = {
	program: string?,
	agent: boolean?,
	dark: boolean?,
	size: string?,
	tone: string?,
}

-- A picture's side in pixels, to match an icon of that text size.
local PIXELS: { [string]: number } = { small = 12, normal = 14, large = 16 }

-- Programs Moonframe has a logo for.
local LOGOS: { [string]: string } = {
	amp = "agent-amp",
	claude = "agent-claude",
	codex = "agent-codex",
	copilot = "agent-copilot",
	cursor = "agent-cursor",
	droid = "agent-droid",
	gemini = "agent-gemini",
	grok = "agent-grok",
	kiro = "agent-kiro",
	opencode = "agent-opencode",
	qwen = "agent-qwen",
}

return component {
	title = "Program Icon",
	category = "Status",
	icon = "square-terminal",
	description = "The icon of what runs in a terminal: an agent's logo, a terminal for a shell.",
	when = "Beside a tab's, pane's or menu row's program name.",
	props = {
		["program"] = "the program's key from app.core: claude, codex, pi, shell, … (\"\" for one the app does not know)",
		["agent"] = "for an unknown program: whether it is an agent (a bot) rather than a terminal",
		["dark"] = "app.dark, for the marks drawn as pictures",
		["size"] = "small | normal | large, as text (default normal)",
		["tone"] = "the icon's tone (pictures keep their colours)",
	},
	example = { program = "claude" },

	build = function(props: Props)
		local program = props.program or ""
		local size = props.size or "normal"
		if program == "pi" then
			-- Mods draw it from app/mods/<name>, so the path climbs to app/icons.
			local src = if props.dark then "../../icons/pi-dark.svg" else "../../icons/pi-light.svg"
			local side = PIXELS[size] or 14
			return ui.image { src, width = side, height = side }
		end
		local name = LOGOS[program] or (if program == "shell" then "square-terminal"
			elseif props.agent then "bot"
			else "terminal")
		return ui.icon { name = name :: any, tone = props.tone :: any, size = size :: any }
	end,
}
