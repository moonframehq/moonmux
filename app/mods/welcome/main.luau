-- Welcome: a short tour on first launch (then remembered in its state,
-- which Moonframe saves), reopened from Help > Welcome to Herdr. Four
-- pages, with Appearance on the first, page dots, Back, Next and Done;
-- ← → and Enter move through it.

local onboarding = use "onboarding"
local appearance = use "appearance"

type State = { seen: boolean, open: boolean, page: number }

local PAGES = {
	{
		icon = "terminal",
		title = "Herdr runs your terminals in the background",
		text = "The terminals and agents you open here don't live in this window. They run in herdr-core, a small background program on your computer. Close Herdr, update it, or let it crash, and your agents keep working. Open it again and everything is where you left it, as long as the computer is on.",
	},
	{
		icon = "bot",
		title = "Agents tell you when they need you",
		text = "A ring while one works, \u{201C}Needs you\u{201D} when it asks something, and Done when it finishes while you look elsewhere. ⌘J peeks at the one that needs you.",
	},
	{
		icon = "messages-square",
		title = "Agents can start helpers",
		text = "When an agent starts a helper in herdr-core, it shows in the sidebar under Elsewhere, with agents started outside Herdr. Click one to peek at it and answer it.",
	},
	{
		icon = "layers",
		title = "Your other machines, in the same window",
		text = "A server or a Mac mini runs its own herdr-core, so its agents keep going even while this laptop sleeps. Add one with `hc machine add <host>`; Settings › Machines lists them.",
	},
}

return mod {
	title = "Welcome",

	styles = {
		[".wl-appearance"] = { padding = { 12, 0, 0, 0 }, gap = 12, border = "top", border_color = "border" },
		[".wl-label"] = { color = "muted-foreground", width = 90 },
	},

	init = function(previous: State?): State
		local seen = previous ~= nil and previous.seen == true
		return { seen = seen, open = not seen, page = 1 }
	end,

	update = function(state: State, msg)
		if msg == "open" then
			state.open, state.page = true, 1
		elseif msg == "next" then
			state.page = math.min(state.page + 1, #PAGES)
		elseif msg == "back" then
			state.page = math.max(state.page - 1, 1)
		elseif msg == "done" then
			state.open, state.seen = false, true
		elseif type(msg) == "table" and msg.page then
			state.page = math.clamp(msg.page, 1, #PAGES)
		else
			return
		end
		return state
	end,

	view = function(state: State, app)
		local pages = table.clone(PAGES)
		pages[1] = table.clone(PAGES[1])
		pages[1].content = {
			ui.row {
				class = "wl-appearance",
				align = "center",
				ui.text { "Appearance", class = "wl-label" },
				appearance { dark = app.dark },
			},
		}
		return onboarding {
			open = state.open,
			title = "Welcome to Herdr",
			icon = "sparkles",
			badge = "BETA",
			pages = pages,
			current = state.page,
			done_label = "Done",
			width = 560,
		}
	end,
}
