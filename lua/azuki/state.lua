--- azuki.nvim state management module
--- Centralized input state management

local M = {}

--- Default state values
local defaults = {
  enabled = false,
  romaji_buffer = "",
  hiragana = "",
  candidates = {},
  selected_index = 0,
  segments = {},
  current_segment = 1,
  preedit_start_col = 0,
  preedit_start_row = 0,
  bufnr = nil,
  last_seq = 0,
}

--- Current state
M.data = vim.deepcopy(defaults)

--- Reset all state to defaults
function M.reset()
  M.data = vim.deepcopy(defaults)
end

--- Reset conversion-related state only (keep enabled, bufnr, preedit position)
function M.reset_conversion()
  M.data.romaji_buffer = ""
  M.data.hiragana = ""
  M.data.candidates = {}
  M.data.selected_index = 0
  M.data.segments = {}
  M.data.current_segment = 1
  M.data.last_seq = 0
end

--- Reset candidates and segments only (keep romaji and hiragana)
function M.clear_candidates()
  M.data.candidates = {}
  M.data.selected_index = 0
  M.data.segments = {}
  M.data.current_segment = 1
end

--- Update preedit anchor position
--- @param row number 0-indexed row
--- @param col number 0-indexed column
function M.set_preedit_anchor(row, col)
  M.data.preedit_start_row = row
  M.data.preedit_start_col = col
end

--- Get combined text from segments
--- @return string Combined text from all segments
function M.get_segments_text()
  local parts = {}
  for _, seg in ipairs(M.data.segments) do
    local idx = seg.selected_index or 1
    local text = seg.candidates[idx] or seg.reading
    table.insert(parts, text)
  end
  return table.concat(parts)
end

--- Get the text to display (segments, candidate, or hiragana + romaji)
--- @return string display_text
--- @return string mode "segments" | "candidate" | "preedit"
function M.get_display_text()
  if #M.data.segments > 0 then
    return M.get_segments_text() .. M.data.romaji_buffer, "segments"
  elseif M.data.selected_index > 0 and #M.data.candidates > 0 then
    return M.data.candidates[M.data.selected_index] .. M.data.romaji_buffer, "candidate"
  else
    return M.data.hiragana .. M.data.romaji_buffer, "preedit"
  end
end

--- Get the text to commit
--- @return string base_text Text without romaji buffer
--- @return string full_text Text with romaji buffer
function M.get_commit_text()
  local base_text
  if #M.data.segments > 0 then
    base_text = M.get_segments_text()
  elseif M.data.selected_index > 0 and #M.data.candidates > 0 then
    base_text = M.data.candidates[M.data.selected_index]
  else
    base_text = M.data.hiragana
  end
  return base_text, base_text .. M.data.romaji_buffer
end

--- Check if there's any preedit content
--- @return boolean
function M.has_preedit()
  return M.data.hiragana ~= "" or M.data.romaji_buffer ~= ""
end

--- Check if there's a selected candidate
--- @return boolean
function M.has_selection()
  return M.data.selected_index > 0 and #M.data.candidates > 0
end

--- Check if in segment mode
--- @return boolean
function M.has_segments()
  return #M.data.segments > 0
end

return M
