-- The Files panel: the focused pane's folder as a lazy tree, as herdr-app's
-- shows it. The shell mounts it on the right (`files#right`) while shown.
--
-- The folder is the focused pane's (the current tab's active one, from
-- `topic:herdr/workspace`): where herdr-core says its shell is, else its
-- space's folder. The `folder` provider (src/folder.rs) lists it and each
-- open folder, marks git's changes, and reads a file for the preview sheet,
-- answering here.

local file_icon = use "file-icon"
local code_view = use "code-view"
local empty_state = use "empty-state"
local sheet = use "sheet"

-- Folders open per folder followed, kept for this many folders.
local REMEMBERED = 20
local INDENT = 14
local ROW_HEIGHT = 26

type Preview = { path: string, text: string?, error: string?, truncated: boolean? }
type State = {
	-- The focused pane, from the workspace, and where its shell is.
	focus: { pane: string, cwd: string }?,
	cwds: { [string]: string },
	-- The folder followed, as given to the provider.
	folder: string?,
	-- Open folders by folder followed: { [folder] = { [path] = true } }.
	open: { [string]: { [string]: boolean } },
	-- The order folders were followed in, oldest first, to forget the
	-- oldest.
	seen: { string },
	-- The file shown in the sheet, while it shows.
	preview: Preview?,
}

-- The folder of the focused pane: its shell's, else its space's.
local function folder_of(state: State): string?
	local focus = state.focus
	if not focus then
		return nil
	end
	local cwd = state.cwds[focus.pane]
	if cwd and cwd ~= "" then
		return cwd
	end
	return if focus.cwd ~= "" then focus.cwd else nil
end

-- The folder's last part: `~/code/herdr` is `herdr`.
local function base(path: string): string
	return string.match(path, "([^/]+)/*$") or path
end

-- Tells the provider the folder and which of its folders are open.
local function follow(state: State): { any }
	local folder = state.folder
	if not folder then
		return {}
	end
	local paths = {}
	local open: { [string]: boolean } = state.open[folder] or {}
	for path in open do
		table.insert(paths, path)
	end
	table.sort(paths)
	-- A list, not lines: a folder's name may hold a newline.
	return { { to = "provider:folder", msg = { root = folder, open = paths } } }
end

-- Follows the focused pane's folder when it moved.
local function refollow(state: State): { any }?
	local folder = folder_of(state)
	if folder == state.folder then
		return nil
	end
	state.folder = folder
	state.preview = nil
	if folder then
		local at = table.find(state.seen, folder)
		if at then
			table.remove(state.seen, at)
		end
		table.insert(state.seen, folder)
		while #state.seen > REMEMBERED do
			state.open[table.remove(state.seen, 1) :: string] = nil
		end
	end
	return follow(state)
end

type Line = { path: string, depth: number, entry: any?, open: boolean? }

local function line(at: Line, marks: { [string]: string })
	local entry = at.entry
	if not entry then
		return ui.row {
			class = "files-row",
			align = "center",
			ui.row { width = INDENT * at.depth + 16 },
			ui.text { "…", tone = "muted", size = "small" },
		}
	end
	local mark = marks[at.path]
	return ui.row {
		class = "files-row",
		align = "center",
		msg = if entry.dir then { toggle = at.path } else { open = at.path },
		ui.row { width = INDENT * at.depth },
		if entry.dir
			then ui.icon { name = if at.open then "chevron-down" else "chevron-right", class = "files-chevron", size = "small" }
			else ui.row { width = 14 },
		file_icon { name = entry.name, dir = entry.dir, open = at.open },
		ui.row { grow = true, ui.text { entry.name, truncate = true } },
		mark and ui.text { mark, class = { "files-mark", `files-mark-{string.lower(mark)}` } },
	}
end

return mod {
	title = "Files",
	subscribe = { "herdr/workspace", "provider/core" },

	styles = {
		[".files-frame"] = { padding = { 0, 10, 0, 0 } },
		[".files-card"] = {
			background = "card",
			radius = 12,
			border = true,
			border_color = "border",
			shadow = "0 1px 2px #0000000d, 0 6px 20px -6px #00000026",
			clip = true,
		},
		[".files-head"] = { padding = { 0, 14 }, gap = 8, border = "bottom", border_color = "border" },
		[".files-folder"] = { size = "small", color = "muted-foreground" },
		[".files-body"] = { padding = 6 },
		[".files-row"] = { padding = { 0, 8 }, radius = 7, gap = 6 },
		[".files-row:hover"] = { background = "list-hover" },
		[".files-chevron"] = { color = "muted-foreground" },
		[".files-mark"] = { size = "small", color = "warning" },
		[".files-mark-a"] = { color = "success" },
		[".files-mark-d"] = { color = "danger" },
		[".files-mark-u"] = { color = "danger" },
		[".files-note"] = { padding = 8 },
	},

	init = function(previous: any): (State, { any }?)
		local state: State = {
			focus = nil,
			cwds = {},
			folder = nil,
			open = previous and previous.open or {},
			seen = previous and previous.seen or {},
			preview = nil,
		}
		-- The topics, heard at once, say which folder to follow.
		return state
	end,

	update = function(state: State, msg)
		if type(msg) ~= "table" then
			if msg == "close-preview" then
				state.preview = nil
				return state
			end
			return nil
		end
		if msg.topic == "herdr/workspace" then
			state.focus = msg.value and msg.value.focus
			return state, refollow(state)
		elseif msg.topic == "provider/core" then
			-- Where each of our panes' shells is now.
			local cwds = {}
			for _, terminal in (msg.value and msg.value.terminals) or {} do
				if terminal.ours and terminal.pane ~= "" then
					cwds[terminal.pane] = terminal.cwd
				end
			end
			state.cwds = cwds
			return state, refollow(state)
		elseif msg.toggle and state.folder then
			local open: { [string]: boolean } = state.open[state.folder] or {}
			if open[msg.toggle] then
				-- Folding a folder folds those inside it too.
				for path in open do
					if path == msg.toggle or string.sub(path, 1, #msg.toggle + 1) == msg.toggle .. "/" then
						open[path] = nil
					end
				end
			else
				open[msg.toggle] = true
			end
			state.open[state.folder] = open
			return state, follow(state)
		elseif msg.open then
			state.preview = { path = msg.open }
			return state, { { to = "provider:folder", msg = { read = msg.open, id = "preview" } } }
		elseif msg.reply == "preview" then
			local preview = state.preview
			-- Only the answer for the file still shown.
			if preview and preview.path == msg.path then
				if msg.ok then
					preview.text = msg.text
					preview.truncated = msg.truncated
				else
					preview.error = msg.error
				end
			end
			return state
		end
		return nil
	end,

	view = function(state: State, app)
		local data = app.folder
		local folder = state.folder
		-- What the provider lists, once it follows this folder.
		local listing = if data and data.folder == folder then data else nil
		local body: any
		if not folder then
			body = empty_state { icon = "folder", title = "No folder", message = "Focus a pane to see its files." }
		elseif not listing then
			body = ui.text { "Loading files…", tone = "muted", size = "small", class = "files-note" }
		elseif not listing.available then
			body = empty_state { icon = "folder", title = "No files", message = listing.reason or "This folder is not there." }
		else
			local open: { [string]: boolean } = state.open[folder] or {}
			local marks = listing.marks or {}
			local lines: { Line } = {}
			local function add(dir: string, depth: number)
				local entries = listing.dirs[dir]
				if not entries then
					-- Opened, and not listed yet.
					table.insert(lines, { path = dir, depth = depth })
					return
				end
				for _, entry in entries do
					local path = if dir == "" then entry.name else `{dir}/{entry.name}`
					local is_open = entry.dir and open[path] == true
					table.insert(lines, { path = path, depth = depth, entry = entry, open = is_open })
					if is_open then
						add(path, depth + 1)
					end
				end
			end
			add("", 0)
			body = ui.column {
				grow = true,
				gap = "none",
				#lines == 0 and ui.text { "No files", tone = "muted", size = "small", class = "files-note" },
				ui.list {
					id = "files",
					count = #lines,
					row_height = ROW_HEIGHT,
					row = function(index)
						return line(lines[index], marks)
					end,
				},
				listing.limited and ui.text {
					"Showing up to 500 entries per folder and 2,000 in all",
					tone = "muted",
					size = "small",
					class = "files-note",
				},
			}
		end

		local preview = state.preview
		local shown: any = false
		if preview then
			shown = if preview.text
				then code_view { text = preview.text, grow = true }
				else ui.text { preview.error or "Reading…", tone = "muted", size = "small", class = "files-note" }
		end

		return ui.column {
			grow = true,
			gap = "none",
			class = "files-frame",
			-- Level with the tab strip, which is the window's title bar.
			ui.row { height = 46, drag = true, shrink = false },
			ui.column {
				class = "files-card",
				grow = true,
				gap = "none",
				ui.row {
					class = "files-head",
					height = 42,
					shrink = false,
					align = "center",
					ui.text { "Files", weight = "semibold" },
					ui.spacer(),
					folder and ui.text { base(folder), class = "files-folder", truncate = true, tooltip = folder },
				},
				ui.column { class = "files-body", grow = true, gap = "none", body },
			},
			sheet {
				open = preview ~= nil,
				dismiss = "close-preview",
				title = if preview then base(preview.path) else "",
				subtitle = if preview
					then (if preview.truncated then `{preview.path} · the start only` else preview.path)
					else nil,
				icon = "file-text",
				width = 640,
				shown,
			},
		}
	end,
}
