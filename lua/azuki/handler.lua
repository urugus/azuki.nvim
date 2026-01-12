--- azuki.nvim handler module
--- Action handlers for input processing

local M = {}

local config = require("azuki.config")
local romaji = require("azuki.romaji")
local state = require("azuki.state")
local server = require("azuki.server")
local ui = require("azuki.ui")

--- Debounce timer
local debounce_timer = nil

--- Callbacks for mode changes (set by input.lua)
M.on_disable = nil

--- Cancel debounce timer
local function cancel_debounce()
  if debounce_timer then
    debounce_timer:stop()
    debounce_timer:close()
    debounce_timer = nil
  end
end

--- Update display based on current state
local function update_display()
  local bufnr = state.data.bufnr
  if not bufnr then
    return
  end

  if state.has_segments() then
    ui.show_segments(
      bufnr,
      state.data.preedit_start_row,
      state.data.preedit_start_col,
      state.data.segments,
      state.data.current_segment,
      state.data.romaji_buffer
    )
  elseif state.has_selection() then
    local display_text = state.data.candidates[state.data.selected_index] .. state.data.romaji_buffer
    ui.show_candidate(bufnr, state.data.preedit_start_row, state.data.preedit_start_col, display_text, true)
  else
    local display_text = state.data.hiragana .. state.data.romaji_buffer
    ui.show_preedit(bufnr, state.data.preedit_start_row, state.data.preedit_start_col, display_text)
  end
end

--- Request conversion from server
local function request_conversion()
  if state.data.hiragana == "" then
    return
  end

  local current_seq = server.get_seq() + 1
  state.data.last_seq = current_seq

  server.convert(state.data.hiragana, { live = true }, function(response)
    if response.seq ~= state.data.last_seq then
      return
    end

    if response.type == "convert_result" then
      if response.segments and #response.segments > 0 then
        state.data.segments = response.segments
        state.data.current_segment = 1
        for _, seg in ipairs(state.data.segments) do
          seg.selected_index = 1
        end
      else
        state.data.segments = {}
        state.data.current_segment = 1
      end

      state.data.candidates = response.candidates or {}
      state.data.selected_index = #state.data.candidates > 0 and 1 or 0

      update_display()
    end
  end)
end

--- Request conversion with debounce
local function request_conversion_debounced()
  cancel_debounce()

  debounce_timer = vim.uv.new_timer()
  debounce_timer:start(
    config.get("debounce_ms"),
    0,
    vim.schedule_wrap(function()
      request_conversion()
    end)
  )
end

--- Insert text into buffer and update cursor
--- @param text string Text to insert
local function insert_text(text)
  local bufnr = state.data.bufnr
  if not bufnr or text == "" then
    return
  end

  ui.clear(bufnr)

  local row = state.data.preedit_start_row
  local col = state.data.preedit_start_col
  local lines = vim.api.nvim_buf_get_lines(bufnr, row, row + 1, false)
  local line = lines[1] or ""

  local before = line:sub(1, col)
  local after = line:sub(col + 1)
  vim.api.nvim_buf_set_lines(bufnr, row, row + 1, false, { before .. text .. after })

  local new_col = col + #text
  vim.api.nvim_win_set_cursor(0, { row + 1, new_col })

  return new_col
end

--- Commit selected candidate (internal, for auto-commit on continued input)
local function commit_selected()
  local base_text, _ = state.get_commit_text()
  if base_text == "" then
    return
  end

  local new_col = insert_text(base_text)

  if state.data.hiragana ~= "" then
    server.commit(state.data.hiragana, base_text, nil)
  end

  -- Reset but keep romaji_buffer for continued input
  local saved_romaji = state.data.romaji_buffer
  state.data.hiragana = ""
  state.data.candidates = {}
  state.data.selected_index = 0
  state.data.segments = {}
  state.data.current_segment = 1
  state.data.preedit_start_row = state.data.preedit_start_row
  state.data.preedit_start_col = new_col
  state.data.romaji_buffer = saved_romaji
end

--- Handle character input
--- @param key string Input key
function M.input(key)
  -- Auto-commit if there's a selection
  if state.has_selection() then
    commit_selected()
  end

  state.data.romaji_buffer = state.data.romaji_buffer .. key

  local hiragana, remaining = romaji.convert(state.data.romaji_buffer)
  state.data.hiragana = state.data.hiragana .. hiragana
  state.data.romaji_buffer = remaining

  state.clear_candidates()
  update_display()

  if config.get("live_conversion") and state.data.hiragana ~= "" then
    request_conversion_debounced()
  end
end

--- Commit the current preedit text
function M.commit()
  local bufnr = state.data.bufnr
  if not bufnr then
    return
  end

  cancel_debounce()

  local base_text, full_text = state.get_commit_text()

  if full_text == "" then
    vim.api.nvim_feedkeys(vim.api.nvim_replace_termcodes("<CR>", true, false, true), "n", false)
    vim.schedule(function()
      local cursor = vim.api.nvim_win_get_cursor(0)
      state.set_preedit_anchor(cursor[1] - 1, cursor[2])
    end)
    return
  end

  local new_col = insert_text(full_text)

  if state.data.hiragana ~= "" then
    server.commit(state.data.hiragana, base_text, nil)
  end

  state.data.romaji_buffer = ""
  state.data.hiragana = ""
  state.data.candidates = {}
  state.data.selected_index = 0
  state.data.segments = {}
  state.data.current_segment = 1
  state.data.preedit_start_row = state.data.preedit_start_row
  state.data.preedit_start_col = new_col
end

--- Handle backspace
function M.backspace()
  cancel_debounce()

  if state.data.romaji_buffer ~= "" then
    state.data.romaji_buffer = state.data.romaji_buffer:sub(1, -2)
  elseif state.data.hiragana ~= "" then
    local chars = vim.fn.split(state.data.hiragana, "\\zs")
    table.remove(chars)
    state.data.hiragana = table.concat(chars)
    state.clear_candidates()
  else
    vim.api.nvim_feedkeys(vim.api.nvim_replace_termcodes("<BS>", true, false, true), "n", false)
    return
  end

  update_display()

  if config.get("live_conversion") and state.data.hiragana ~= "" then
    request_conversion_debounced()
  end
end

--- Handle escape (disable input mode)
function M.escape()
  if M.on_disable then
    M.on_disable()
  end
  vim.cmd("stopinsert")
end

--- Cancel conversion (revert to hiragana)
function M.cancel()
  cancel_debounce()
  state.data.last_seq = server.get_seq() + 1000
  state.clear_candidates()
  update_display()
end

--- Select next candidate
function M.next_candidate()
  if state.data.hiragana == "" then
    vim.api.nvim_feedkeys(" ", "n", false)
    return
  end

  if state.has_segments() then
    local seg = state.data.segments[state.data.current_segment]
    if seg and #seg.candidates > 0 then
      seg.selected_index = (seg.selected_index or 1) % #seg.candidates + 1
      update_display()
    end
    return
  end

  if #state.data.candidates == 0 then
    cancel_debounce()
    request_conversion()
    return
  end

  state.data.selected_index = state.data.selected_index % #state.data.candidates + 1
  update_display()
end

--- Select previous candidate
function M.prev_candidate()
  if state.has_segments() then
    local seg = state.data.segments[state.data.current_segment]
    if seg and #seg.candidates > 0 then
      local idx = (seg.selected_index or 1) - 1
      if idx < 1 then
        idx = #seg.candidates
      end
      seg.selected_index = idx
      update_display()
    end
    return
  end

  if #state.data.candidates == 0 then
    return
  end

  state.data.selected_index = state.data.selected_index - 1
  if state.data.selected_index < 1 then
    state.data.selected_index = #state.data.candidates
  end
  update_display()
end

--- Move to next segment
function M.next_segment()
  if not state.has_segments() then
    return
  end

  if state.data.current_segment < #state.data.segments then
    state.data.current_segment = state.data.current_segment + 1
    update_display()
  end
end

--- Move to previous segment
function M.prev_segment()
  if not state.has_segments() then
    return
  end

  if state.data.current_segment > 1 then
    state.data.current_segment = state.data.current_segment - 1
    update_display()
  end
end

--- Shrink current segment
function M.shrink_segment()
  if not state.has_segments() then
    return
  end

  local seg = state.data.segments[state.data.current_segment]
  if not seg or seg.length <= 1 then
    return
  end

  server.adjust_segment(
    state.data.hiragana,
    state.data.segments,
    state.data.current_segment - 1,
    "shrink",
    function(response)
      if response.type == "adjust_segment_result" then
        state.data.segments = response.segments
        for _, s in ipairs(state.data.segments) do
          s.selected_index = 1
        end
        update_display()
      end
    end
  )
end

--- Extend current segment
function M.extend_segment()
  if not state.has_segments() then
    return
  end

  if state.data.current_segment >= #state.data.segments then
    return
  end

  local next_seg = state.data.segments[state.data.current_segment + 1]
  if not next_seg or next_seg.length <= 1 then
    return
  end

  server.adjust_segment(
    state.data.hiragana,
    state.data.segments,
    state.data.current_segment - 1,
    "extend",
    function(response)
      if response.type == "adjust_segment_result" then
        state.data.segments = response.segments
        for _, s in ipairs(state.data.segments) do
          s.selected_index = 1
        end
        update_display()
      end
    end
  )
end

--- Cancel debounce (exposed for cleanup)
function M.cancel_debounce()
  cancel_debounce()
end

--- Update display (exposed for external use)
function M.update_display()
  update_display()
end

return M
