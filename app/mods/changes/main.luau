-- The Changes panel: the uncommitted changes of the focused pane's
-- repository, as herdr-app's shows them. The shell mounts it on the right
-- (`changes#right`) while shown.
--
-- The folder is the focused pane's (the current tab's active one, from
-- `topic:moonmux/workspace`): where herdr-core says its shell is, else its
-- space's folder. The `changes` provider (src/changes.rs) reads its
-- repository, the chosen file's diff, and commits, answering here.

local change_tree = use "change-tree"
local diff_view = use "diff-view"
local empty_state = use "empty-state"

type State = {
	-- The focused pane, from the workspace, and where its shell is.
	focus: { pane: string, cwd: string }?,
	cwds: { [string]: string },
	-- The folder followed, as given to the provider.
	folder: string?,
	-- The file whose diff shows, and folders folded shut.
	selected: string?,
	folded: { [string]: boolean },
	-- The commit message being written.
	message: string,
	-- A commit runs, and what the last one said.
	committing: boolean,
	note: { ok: boolean, text: string }?,
}

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

-- Follows the focused pane's folder when it moved; another repository
-- starts with no file chosen.
local function refollow(state: State): { any }?
	local folder = folder_of(state)
	if folder == state.folder then
		return nil
	end
	state.folder = folder
	state.selected = nil
	state.folded = {}
	state.note = nil
	if not folder then
		return nil
	end
	return { { to = "provider:changes", msg = { root = folder, diff = false } } }
end

local function trimmed(text: string): string
	return string.match(text, "^%s*(.-)%s*$") or ""
end

return mod {
	title = "Changes",
	subscribe = { "moonmux/workspace", "provider/core" },

	styles = {
		[".changes-frame"] = { padding = { 0, 10, 0, 0 } },
		[".changes-card"] = {
			background = "card",
			radius = 12,
			border = true,
			border_color = "border",
			shadow = "0 1px 2px #0000000d, 0 6px 20px -6px #00000026",
			clip = true,
		},
		[".changes-head"] = { padding = { 0, 14 }, gap = 8, border = "bottom", border_color = "border" },
		[".changes-count"] = {
			size = "small",
			color = "muted-foreground",
			border = true,
			border_color = "border",
			radius = 4,
			padding = { 0, 5 },
		},
		[".changes-list"] = { padding = 6 },
		[".changes-note"] = { padding = { 6, 12 } },
		[".changes-foot"] = { padding = 10, gap = 8, border = "top", border_color = "border" },
	},

	init = function(previous: any): State
		-- The message survives a relaunch; the rest comes from the topics.
		return {
			focus = nil,
			cwds = {},
			folder = nil,
			selected = nil,
			folded = {},
			message = previous and previous.message or "",
			committing = false,
			note = nil,
		}
	end,

	update = function(state: State, msg)
		if msg == "commit" then
			local message = trimmed(state.message)
			if message == "" or state.committing or not state.folder then
				return nil
			end
			state.committing = true
			state.note = nil
			return state, { { to = "provider:changes", msg = { commit = message, id = "commit" } } }
		end
		if type(msg) ~= "table" then
			return nil
		end
		if msg.topic == "moonmux/workspace" then
			state.focus = msg.value and msg.value.focus
			return state, refollow(state)
		elseif msg.topic == "provider/core" then
			local cwds = {}
			for _, terminal in (msg.value and msg.value.terminals) or {} do
				if terminal.ours and terminal.pane ~= "" then
					cwds[terminal.pane] = terminal.cwd
				end
			end
			state.cwds = cwds
			return state, refollow(state)
		elseif msg.msg == "message" then
			state.message = msg.value
			return state
		elseif msg.pick then
			-- Choosing the open file again closes its diff.
			state.selected = if state.selected == msg.pick then nil else msg.pick
			return state, { { to = "provider:changes", msg = { diff = state.selected or false } } }
		elseif msg.fold then
			state.folded[msg.fold] = (if state.folded[msg.fold] then nil else true) :: any
			return state
		elseif msg.reply == "commit" then
			state.committing = false
			if msg.ok then
				state.message = ""
				state.selected = nil
				state.note = nil
			else
				state.note = { ok = false, text = msg.error or "The commit failed" }
			end
			return state
		end
		return nil
	end,

	view = function(state: State, app)
		local data = app.changes
		local folder = state.folder
		local repo = if data and data.folder == folder then data else nil
		local ready = repo ~= nil and repo.repository == true
		local changed = if ready then repo.changed else 0

		local body: any
		if not folder then
			body = empty_state { icon = "git-compare", title = "No folder", message = "Focus a pane to see its changes." }
		elseif not repo then
			body = ui.text { "Loading changes…", tone = "muted", size = "small", class = "changes-note" }
		elseif not repo.repository then
			body = empty_state { icon = "git-branch", title = "Not a git repository", message = folder }
		elseif changed == 0 then
			body = empty_state { icon = "circle-check", title = "No uncommitted changes", message = `{repo.branch} is clean.` }
		else
			local lines: { string } = {}
			local status: { [string]: string } = {}
			for _, file in repo.files do
				table.insert(lines, `{file.status}\t{file.added}\t{file.deleted}\t{file.path}`)
				status[file.path] = file.status
			end
			local list = ui.column {
				class = "changes-list",
				gap = "none",
				change_tree { files = lines, selected = state.selected, folded = state.folded, counts = true },
				repo.truncated and ui.text { "Showing the first 300 changed files", tone = "muted", size = "small" },
			}
			local diff = repo.diff
			local selected = state.selected
			if selected and status[selected] then
				local shown: any
				if diff and diff.path == selected then
					if diff.binary then
						shown = ui.text { "Binary file · no text diff", tone = "muted", size = "small", class = "changes-note" }
					else
						shown = ui.column {
							grow = true,
							gap = "none",
							diff_view {
								path = diff.path,
								status = status[selected],
								hunks = diff.hunks,
								grow = true,
								actions = {
									ui.button { icon = "close", variant = "ghost", small = true, msg = { pick = selected }, tooltip = "Close the diff" },
								},
							},
							diff.truncated and ui.text {
								"The diff is cut short",
								tone = "muted",
								size = "small",
								class = "changes-note",
							},
						}
					end
				else
					shown = ui.text { "Reading the diff…", tone = "muted", size = "small", class = "changes-note" }
				end
				body = ui.vsplit {
					id = "changes",
					ui.pane { size = 200, min = 60, scroll = true, list },
					ui.pane { min = 120, shown },
				}
			else
				body = ui.column {
					grow = true,
					gap = "none",
					ui.column { scroll = true, grow = true, gap = "none", list },
					ui.text { "Select a file to view its changes", tone = "muted", size = "small", class = "changes-note" },
				}
			end
		end

		local can_commit = ready and changed > 0 and trimmed(state.message) ~= "" and not state.committing
		local note = state.note
		local foot = ui.column {
			gap = "none",
			shrink = false,
			note and ui.text { note.text, tone = if note.ok then "success" else "danger", size = "small", class = "changes-note", wrap = true },
			ui.row {
				class = "changes-foot",
				align = "center",
				ui.input {
					id = "message",
					value = state.message,
					placeholder = "Commit message",
					change = "message",
					submit = "commit",
					grow = true,
					disabled = not ready or state.committing,
				},
				ui.button {
					if state.committing then "Committing…" else "Commit",
					variant = "primary",
					disabled = not can_commit,
					msg = "commit",
				},
			},
		}

		return ui.column {
			grow = true,
			gap = "none",
			class = "changes-frame",
			-- Level with the tab strip, which is the window's title bar.
			ui.row { height = 46, drag = true, shrink = false },
			ui.column {
				class = "changes-card",
				grow = true,
				gap = "none",
				ui.row {
					class = "changes-head",
					height = 42,
					shrink = false,
					align = "center",
					ui.text { "Changes", weight = "semibold" },
					ready and ui.text { `{changed} {if changed == 1 then "file" else "files"}`, class = "changes-count" },
					ui.spacer(),
					-- A new file longer than what is read counts its start only.
					ready and ui.text {
						if repo.partial then `+{repo.added}…` else `+{repo.added}`,
						size = "small",
						tone = "success",
						tooltip = if repo.partial then "Large new files count their first 256 KiB" else nil,
					},
					ready and ui.text { `−{repo.deleted}`, size = "small", tone = "danger" },
				},
				ui.column { grow = true, gap = "none", body },
				foot,
			},
		}
	end,
}
