--- azuki.nvim UI module
--- Handles display of preedit text and candidates using extmarks

local M = {}

local config = require("azuki.config")

--- Namespace ID for extmarks
M.ns_id = nil

--- Current extmark ID
M.current_mark_id = nil

--- Initialize the UI module
function M.setup()
  M.ns_id = vim.api.nvim_create_namespace("azuki")
end

--- Show preedit text (unconverted/converting text)
--- @param bufnr number Buffer number
--- @param row number Row number (0-indexed)
--- @param col number Column number (0-indexed, cursor position)
--- @param text string Text to display
function M.show_preedit(bufnr, row, col, text)
  -- Clear existing extmark
  M.clear(bufnr)

  if text == "" then
    return
  end

  -- Get highlight group from config
  local hl_group = config.get("highlight").pending

  -- Set extmark with inline virtual text
  M.current_mark_id = vim.api.nvim_buf_set_extmark(bufnr, M.ns_id, row, col, {
    virt_text = { { text, hl_group } },
    virt_text_pos = "inline",
    right_gravity = true,
  })
end

--- Show conversion candidate (replaces preedit)
--- @param bufnr number Buffer number
--- @param row number Row number (0-indexed)
--- @param col number Column number (0-indexed)
--- @param candidate string The candidate text
--- @param is_selected boolean Whether this candidate is selected
function M.show_candidate(bufnr, row, col, candidate, is_selected)
  M.clear(bufnr)

  if candidate == "" then
    return
  end

  local hl = config.get("highlight")
  local hl_group = is_selected and hl.selected or hl.pending

  M.current_mark_id = vim.api.nvim_buf_set_extmark(bufnr, M.ns_id, row, col, {
    virt_text = { { candidate, hl_group } },
    virt_text_pos = "inline",
    right_gravity = true,
  })
end

--- Clear all extmarks in the buffer
--- @param bufnr number Buffer number
function M.clear(bufnr)
  if M.ns_id then
    vim.api.nvim_buf_clear_namespace(bufnr, M.ns_id, 0, -1)
  end
  M.current_mark_id = nil
end

--- Get current extmark info
--- @param bufnr number Buffer number
--- @return table|nil Extmark info or nil
function M.get_current_mark(bufnr)
  if not M.current_mark_id or not M.ns_id then
    return nil
  end

  local marks = vim.api.nvim_buf_get_extmarks(bufnr, M.ns_id, 0, -1, { details = true })
  for _, mark in ipairs(marks) do
    if mark[1] == M.current_mark_id then
      return {
        id = mark[1],
        row = mark[2],
        col = mark[3],
        details = mark[4],
      }
    end
  end

  return nil
end

return M
