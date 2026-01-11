--- azuki.nvim input management module
--- Handles Insert mode key hooks and input state

local M = {}

local config = require("azuki.config")
local romaji = require("azuki.romaji")
local ui = require("azuki.ui")
local server = require("azuki.server")

--- Input state
M.state = {
  enabled = false, -- Japanese input mode enabled
  romaji_buffer = "", -- Romaji input buffer
  hiragana = "", -- Converted hiragana
  candidates = {}, -- Conversion candidates (for fallback)
  selected_index = 0, -- Selected candidate index (1-indexed, 0 = no selection)
  segments = {}, -- Segment information from server
  current_segment = 1, -- Current segment index (1-indexed)
  preedit_start_col = 0, -- Input start column
  preedit_start_row = 0, -- Input start row
  bufnr = nil, -- Current buffer number
  last_seq = 0, -- Last server request sequence number
}

--- Debounce timer
local debounce_timer = nil

--- Enable Japanese input mode
function M.enable()
  if M.state.enabled then
    return
  end

  -- Start server if not running
  if not server.is_active() then
    server.start({ server_path = config.get("server_path") }, function(success)
      if success then
        M._do_enable()
      else
        vim.notify("[azuki] Failed to start server", vim.log.levels.ERROR)
      end
    end)
  else
    M._do_enable()
  end
end

--- Internal: Actually enable input mode
function M._do_enable()
  M.state.enabled = true
  M.state.romaji_buffer = ""
  M.state.hiragana = ""
  M.state.candidates = {}
  M.state.selected_index = 0
  M.state.segments = {}
  M.state.current_segment = 1
  M.state.bufnr = vim.api.nvim_get_current_buf()
  M.state.last_seq = 0

  -- Record current cursor position
  local cursor = vim.api.nvim_win_get_cursor(0)
  M.state.preedit_start_row = cursor[1] - 1 -- 0-indexed
  M.state.preedit_start_col = cursor[2]

  -- Setup key hooks
  M._setup_key_hooks()

  vim.notify("[azuki] Japanese input enabled", vim.log.levels.INFO)
end

--- Disable Japanese input mode
function M.disable()
  if not M.state.enabled then
    return
  end

  -- Cancel any pending debounce timer
  M._cancel_debounce()

  -- Commit any pending text
  if M.state.hiragana ~= "" or M.state.romaji_buffer ~= "" then
    M.commit()
  end

  -- Teardown key hooks
  M._teardown_key_hooks()

  -- Reset state
  M.state.enabled = false
  M.state.romaji_buffer = ""
  M.state.hiragana = ""
  M.state.candidates = {}
  M.state.selected_index = 0
  M.state.segments = {}
  M.state.current_segment = 1
  M.state.last_seq = 0

  -- Clear UI
  if M.state.bufnr then
    ui.clear(M.state.bufnr)
  end
  M.state.bufnr = nil

  vim.notify("[azuki] Japanese input disabled", vim.log.levels.INFO)
end

--- Toggle Japanese input mode
function M.toggle()
  if M.state.enabled then
    M.disable()
  else
    M.enable()
  end
end

--- Setup key hooks for Insert mode
function M._setup_key_hooks()
  local bufnr = M.state.bufnr

  -- Hook alphabet keys (a-z)
  for c = string.byte("a"), string.byte("z") do
    local key = string.char(c)
    M._map_key(bufnr, key)
  end

  -- Hook uppercase (A-Z)
  for c = string.byte("A"), string.byte("Z") do
    local key = string.char(c)
    M._map_key(bufnr, key)
  end

  -- Special characters
  M._map_key(bufnr, "-")
  M._map_key(bufnr, "'")

  -- Control keys
  vim.keymap.set("i", "<CR>", function()
    M.commit()
  end, { buffer = bufnr, noremap = true })

  vim.keymap.set("i", "<BS>", function()
    M.backspace()
  end, { buffer = bufnr, noremap = true })

  vim.keymap.set("i", "<Esc>", function()
    M.disable()
    vim.cmd("stopinsert")
  end, { buffer = bufnr, noremap = true })

  -- Candidate selection keys
  vim.keymap.set("i", "<Space>", function()
    M.next_candidate()
  end, { buffer = bufnr, noremap = true })

  vim.keymap.set("i", "<S-Space>", function()
    M.prev_candidate()
  end, { buffer = bufnr, noremap = true })

  -- Cancel key
  vim.keymap.set("i", "<C-g>", function()
    M.cancel()
  end, { buffer = bufnr, noremap = true })

  -- Segment navigation keys
  vim.keymap.set("i", "<Tab>", function()
    M.next_segment()
  end, { buffer = bufnr, noremap = true })

  vim.keymap.set("i", "<S-Tab>", function()
    M.prev_segment()
  end, { buffer = bufnr, noremap = true })

  -- Segment boundary adjustment keys
  vim.keymap.set("i", "<S-Left>", function()
    M.shrink_segment()
  end, { buffer = bufnr, noremap = true })

  vim.keymap.set("i", "<S-Right>", function()
    M.extend_segment()
  end, { buffer = bufnr, noremap = true })
end

--- Map a single key to input handler
--- @param bufnr number Buffer number
--- @param key string Key to map
function M._map_key(bufnr, key)
  vim.keymap.set("i", key, function()
    M.handle_input(key)
  end, { buffer = bufnr, noremap = true })
end

--- Teardown key hooks
function M._teardown_key_hooks()
  local bufnr = M.state.bufnr
  if not bufnr then
    return
  end

  -- Remove alphabet mappings
  for c = string.byte("a"), string.byte("z") do
    pcall(vim.keymap.del, "i", string.char(c), { buffer = bufnr })
  end
  for c = string.byte("A"), string.byte("Z") do
    pcall(vim.keymap.del, "i", string.char(c), { buffer = bufnr })
  end

  -- Remove special character mappings
  pcall(vim.keymap.del, "i", "-", { buffer = bufnr })
  pcall(vim.keymap.del, "i", "'", { buffer = bufnr })

  -- Remove control key mappings
  pcall(vim.keymap.del, "i", "<CR>", { buffer = bufnr })
  pcall(vim.keymap.del, "i", "<BS>", { buffer = bufnr })
  pcall(vim.keymap.del, "i", "<Esc>", { buffer = bufnr })
  pcall(vim.keymap.del, "i", "<Space>", { buffer = bufnr })
  pcall(vim.keymap.del, "i", "<S-Space>", { buffer = bufnr })
  pcall(vim.keymap.del, "i", "<C-g>", { buffer = bufnr })
  pcall(vim.keymap.del, "i", "<Tab>", { buffer = bufnr })
  pcall(vim.keymap.del, "i", "<S-Tab>", { buffer = bufnr })
  pcall(vim.keymap.del, "i", "<S-Left>", { buffer = bufnr })
  pcall(vim.keymap.del, "i", "<S-Right>", { buffer = bufnr })
end

--- Handle input key
--- @param key string Input key
function M.handle_input(key)
  -- If we have a selected candidate, commit it first before continuing input
  if M.state.selected_index > 0 and #M.state.candidates > 0 then
    M._commit_selected()
  end

  -- Add to romaji buffer
  M.state.romaji_buffer = M.state.romaji_buffer .. key

  -- Convert romaji to hiragana
  local hiragana, remaining = romaji.convert(M.state.romaji_buffer)

  -- Update state
  M.state.hiragana = M.state.hiragana .. hiragana
  M.state.romaji_buffer = remaining

  -- Clear candidates and segments when input changes
  M.state.candidates = {}
  M.state.selected_index = 0
  M.state.segments = {}
  M.state.current_segment = 1

  -- Update display
  M._update_display()

  -- Request live conversion with debounce
  if config.get("live_conversion") and M.state.hiragana ~= "" then
    M._request_conversion_debounced()
  end
end

--- Cancel debounce timer
function M._cancel_debounce()
  if debounce_timer then
    debounce_timer:stop()
    debounce_timer:close()
    debounce_timer = nil
  end
end

--- Request conversion with debounce
function M._request_conversion_debounced()
  M._cancel_debounce()

  debounce_timer = vim.uv.new_timer()
  debounce_timer:start(
    config.get("debounce_ms"),
    0,
    vim.schedule_wrap(function()
      M._request_conversion()
    end)
  )
end

--- Request conversion from server
function M._request_conversion()
  if M.state.hiragana == "" then
    return
  end

  -- Record the sequence number before sending
  local current_seq = server.get_seq() + 1
  M.state.last_seq = current_seq

  server.convert(M.state.hiragana, { live = true }, function(response)
    -- Ignore stale responses
    if response.seq ~= M.state.last_seq then
      return
    end

    if response.type == "convert_result" then
      -- Store segment information
      if response.segments and #response.segments > 0 then
        M.state.segments = response.segments
        M.state.current_segment = 1
        -- Initialize selected_index for each segment to 1
        for _, seg in ipairs(M.state.segments) do
          seg.selected_index = 1
        end
      else
        M.state.segments = {}
        M.state.current_segment = 1
      end

      -- Keep candidates for fallback
      M.state.candidates = response.candidates or {}
      if #M.state.candidates > 0 then
        M.state.selected_index = 1
      else
        M.state.selected_index = 0
      end

      M._update_display()
    end
  end)
end

--- Update display
function M._update_display()
  local bufnr = M.state.bufnr
  if not bufnr then
    return
  end

  if #M.state.segments > 0 then
    -- Show segments
    ui.show_segments(
      bufnr,
      M.state.preedit_start_row,
      M.state.preedit_start_col,
      M.state.segments,
      M.state.current_segment,
      M.state.romaji_buffer
    )
  elseif M.state.selected_index > 0 and #M.state.candidates > 0 then
    -- Fallback: Show selected candidate
    local display_text = M.state.candidates[M.state.selected_index] .. M.state.romaji_buffer
    ui.show_candidate(bufnr, M.state.preedit_start_row, M.state.preedit_start_col, display_text, true)
  else
    -- Show hiragana + pending romaji
    local display_text = M.state.hiragana .. M.state.romaji_buffer
    ui.show_preedit(bufnr, M.state.preedit_start_row, M.state.preedit_start_col, display_text)
  end
end

--- Select next candidate (for current segment or fallback)
function M.next_candidate()
  if M.state.hiragana == "" then
    -- No hiragana to convert, pass through space
    vim.api.nvim_feedkeys(" ", "n", false)
    return
  end

  -- Segment mode: cycle candidate for current segment
  if #M.state.segments > 0 then
    local seg = M.state.segments[M.state.current_segment]
    if seg and #seg.candidates > 0 then
      seg.selected_index = (seg.selected_index or 1) % #seg.candidates + 1
      M._update_display()
    end
    return
  end

  -- Fallback mode: cycle global candidates
  if #M.state.candidates == 0 then
    M._cancel_debounce()
    M._request_conversion()
    return
  end

  M.state.selected_index = M.state.selected_index % #M.state.candidates + 1
  M._update_display()
end

--- Select previous candidate (for current segment or fallback)
function M.prev_candidate()
  -- Segment mode: cycle candidate for current segment
  if #M.state.segments > 0 then
    local seg = M.state.segments[M.state.current_segment]
    if seg and #seg.candidates > 0 then
      local idx = (seg.selected_index or 1) - 1
      if idx < 1 then
        idx = #seg.candidates
      end
      seg.selected_index = idx
      M._update_display()
    end
    return
  end

  -- Fallback mode
  if #M.state.candidates == 0 then
    return
  end

  M.state.selected_index = M.state.selected_index - 1
  if M.state.selected_index < 1 then
    M.state.selected_index = #M.state.candidates
  end

  M._update_display()
end

--- Move to next segment
function M.next_segment()
  if #M.state.segments == 0 then
    return
  end

  if M.state.current_segment < #M.state.segments then
    M.state.current_segment = M.state.current_segment + 1
    M._update_display()
  end
end

--- Move to previous segment
function M.prev_segment()
  if #M.state.segments == 0 then
    return
  end

  if M.state.current_segment > 1 then
    M.state.current_segment = M.state.current_segment - 1
    M._update_display()
  end
end

--- Shrink current segment (move boundary left)
function M.shrink_segment()
  if #M.state.segments == 0 then
    return
  end

  local seg = M.state.segments[M.state.current_segment]
  if not seg or seg.length <= 1 then
    return -- Cannot shrink further
  end

  server.adjust_segment(
    M.state.hiragana,
    M.state.segments,
    M.state.current_segment - 1, -- 0-indexed for server
    "shrink",
    function(response)
      if response.type == "adjust_segment_result" then
        M.state.segments = response.segments
        -- Initialize selected_index for each segment
        for _, s in ipairs(M.state.segments) do
          s.selected_index = 1
        end
        M._update_display()
      end
    end
  )
end

--- Extend current segment (move boundary right)
function M.extend_segment()
  if #M.state.segments == 0 then
    return
  end

  -- Cannot extend if this is the last segment
  if M.state.current_segment >= #M.state.segments then
    return
  end

  -- Cannot extend if next segment has only 1 character
  local next_seg = M.state.segments[M.state.current_segment + 1]
  if not next_seg or next_seg.length <= 1 then
    return
  end

  server.adjust_segment(
    M.state.hiragana,
    M.state.segments,
    M.state.current_segment - 1, -- 0-indexed for server
    "extend",
    function(response)
      if response.type == "adjust_segment_result" then
        M.state.segments = response.segments
        -- Initialize selected_index for each segment
        for _, s in ipairs(M.state.segments) do
          s.selected_index = 1
        end
        M._update_display()
      end
    end
  )
end

--- Cancel conversion (revert to hiragana)
function M.cancel()
  -- Cancel any pending debounce timer
  M._cancel_debounce()

  -- Invalidate any in-flight server responses by incrementing last_seq
  M.state.last_seq = server.get_seq() + 1000

  M.state.candidates = {}
  M.state.selected_index = 0
  M.state.segments = {}
  M.state.current_segment = 1
  M._update_display()
end

--- Get combined text from segments
--- @return string Combined text from all segments
local function get_segments_text()
  local parts = {}
  for _, seg in ipairs(M.state.segments) do
    local idx = seg.selected_index or 1
    local text = seg.candidates[idx] or seg.reading
    table.insert(parts, text)
  end
  return table.concat(parts)
end

--- Commit selected candidate (internal, for auto-commit on input)
function M._commit_selected()
  local bufnr = M.state.bufnr
  if not bufnr then
    return
  end

  local commit_text
  if #M.state.segments > 0 then
    -- Segment mode: combine selected candidates from all segments
    commit_text = get_segments_text()
  elseif M.state.selected_index > 0 and #M.state.candidates > 0 then
    commit_text = M.state.candidates[M.state.selected_index]
  else
    commit_text = M.state.hiragana
  end

  if commit_text == "" then
    return
  end

  -- Clear UI
  ui.clear(bufnr)

  -- Insert text into buffer
  local row = M.state.preedit_start_row
  local col = M.state.preedit_start_col
  local lines = vim.api.nvim_buf_get_lines(bufnr, row, row + 1, false)
  local line = lines[1] or ""

  local before = line:sub(1, col)
  local after = line:sub(col + 1)
  vim.api.nvim_buf_set_lines(bufnr, row, row + 1, false, { before .. commit_text .. after })

  -- Move cursor to end of inserted text
  local new_col = col + #commit_text
  vim.api.nvim_win_set_cursor(0, { row + 1, new_col })

  -- Send commit to server for learning
  if M.state.hiragana ~= "" then
    server.commit(M.state.hiragana, commit_text, nil)
  end

  -- Reset state (keep romaji_buffer for continued input)
  M.state.hiragana = ""
  M.state.candidates = {}
  M.state.selected_index = 0
  M.state.segments = {}
  M.state.current_segment = 1
  M.state.preedit_start_row = row
  M.state.preedit_start_col = new_col
end

--- Commit the current preedit text
function M.commit()
  local bufnr = M.state.bufnr
  if not bufnr then
    return
  end

  -- Cancel any pending debounce timer
  M._cancel_debounce()

  -- Determine text to commit
  local base_text
  if #M.state.segments > 0 then
    -- Segment mode: combine selected candidates from all segments
    base_text = get_segments_text()
  elseif M.state.selected_index > 0 and #M.state.candidates > 0 then
    base_text = M.state.candidates[M.state.selected_index]
  else
    base_text = M.state.hiragana
  end
  local commit_text = base_text .. M.state.romaji_buffer

  if commit_text == "" then
    -- Nothing to commit, pass through Enter
    vim.api.nvim_feedkeys(vim.api.nvim_replace_termcodes("<CR>", true, false, true), "n", false)
    -- Update preedit anchor after Enter is processed
    vim.schedule(function()
      local cursor = vim.api.nvim_win_get_cursor(0)
      M.state.preedit_start_row = cursor[1] - 1
      M.state.preedit_start_col = cursor[2]
    end)
    return
  end

  -- Clear UI
  ui.clear(bufnr)

  -- Insert text into buffer
  local row = M.state.preedit_start_row
  local col = M.state.preedit_start_col
  local lines = vim.api.nvim_buf_get_lines(bufnr, row, row + 1, false)
  local line = lines[1] or ""

  local before = line:sub(1, col)
  local after = line:sub(col + 1)
  vim.api.nvim_buf_set_lines(bufnr, row, row + 1, false, { before .. commit_text .. after })

  -- Move cursor to end of inserted text
  local new_col = col + #commit_text
  vim.api.nvim_win_set_cursor(0, { row + 1, new_col })

  -- Send commit to server for learning
  if M.state.hiragana ~= "" then
    server.commit(M.state.hiragana, base_text, nil)
  end

  -- Reset state (keep mode enabled)
  M.state.romaji_buffer = ""
  M.state.hiragana = ""
  M.state.candidates = {}
  M.state.selected_index = 0
  M.state.segments = {}
  M.state.current_segment = 1
  M.state.preedit_start_row = row
  M.state.preedit_start_col = new_col
end

--- Handle backspace
function M.backspace()
  -- Cancel any pending debounce timer
  M._cancel_debounce()

  if M.state.romaji_buffer ~= "" then
    -- Remove from romaji buffer
    M.state.romaji_buffer = M.state.romaji_buffer:sub(1, -2)
  elseif M.state.hiragana ~= "" then
    -- Remove from hiragana (UTF-8 aware)
    local chars = vim.fn.split(M.state.hiragana, "\\zs")
    table.remove(chars)
    M.state.hiragana = table.concat(chars)
    -- Clear candidates when hiragana changes
    M.state.candidates = {}
    M.state.selected_index = 0
  else
    -- Nothing to delete, pass through backspace
    vim.api.nvim_feedkeys(vim.api.nvim_replace_termcodes("<BS>", true, false, true), "n", false)
    return
  end

  M._update_display()

  -- Request new conversion after backspace
  if config.get("live_conversion") and M.state.hiragana ~= "" then
    M._request_conversion_debounced()
  end
end

--- Check if input mode is enabled
--- @return boolean
function M.is_enabled()
  return M.state.enabled
end

return M
