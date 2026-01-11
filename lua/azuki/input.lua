--- azuki.nvim input management module
--- Handles Insert mode key hooks and input state

local M = {}

local romaji = require("azuki.romaji")
local ui = require("azuki.ui")
local server = require("azuki.server")

--- Input state
M.state = {
  enabled = false, -- Japanese input mode enabled
  romaji_buffer = "", -- Romaji input buffer
  hiragana = "", -- Converted hiragana
  candidates = {}, -- Conversion candidates (Phase 2b)
  selected_index = 0, -- Selected candidate index (Phase 2b)
  preedit_start_col = 0, -- Input start column
  preedit_start_row = 0, -- Input start row
  bufnr = nil, -- Current buffer number
}

--- Enable Japanese input mode
function M.enable()
  if M.state.enabled then
    return
  end

  -- Start server if not running
  if not server.is_active() then
    server.start(nil, function(success)
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
  M.state.bufnr = vim.api.nvim_get_current_buf()

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
end

--- Handle input key
--- @param key string Input key
function M.handle_input(key)
  -- Add to romaji buffer
  M.state.romaji_buffer = M.state.romaji_buffer .. key

  -- Convert romaji to hiragana
  local hiragana, remaining = romaji.convert(M.state.romaji_buffer)

  -- Update state
  M.state.hiragana = M.state.hiragana .. hiragana
  M.state.romaji_buffer = remaining

  -- Update display
  M._update_display()
end

--- Update display
function M._update_display()
  local bufnr = M.state.bufnr
  if not bufnr then
    return
  end

  -- Build display text: hiragana + pending romaji
  local display_text = M.state.hiragana .. M.state.romaji_buffer

  -- Show via extmark
  ui.show_preedit(bufnr, M.state.preedit_start_row, M.state.preedit_start_col, display_text)
end

--- Commit the current preedit text
function M.commit()
  local bufnr = M.state.bufnr
  if not bufnr then
    return
  end

  -- Determine text to commit
  local commit_text = M.state.hiragana .. M.state.romaji_buffer

  if commit_text == "" then
    -- Nothing to commit, pass through Enter
    vim.api.nvim_feedkeys(vim.api.nvim_replace_termcodes("<CR>", true, false, true), "n", false)
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

  -- Reset state (keep mode enabled)
  M.state.romaji_buffer = ""
  M.state.hiragana = ""
  M.state.candidates = {}
  M.state.selected_index = 0
  M.state.preedit_start_row = row
  M.state.preedit_start_col = new_col
end

--- Handle backspace
function M.backspace()
  if M.state.romaji_buffer ~= "" then
    -- Remove from romaji buffer
    M.state.romaji_buffer = M.state.romaji_buffer:sub(1, -2)
  elseif M.state.hiragana ~= "" then
    -- Remove from hiragana (UTF-8 aware)
    local chars = vim.fn.split(M.state.hiragana, "\\zs")
    table.remove(chars)
    M.state.hiragana = table.concat(chars)
  else
    -- Nothing to delete, pass through backspace
    vim.api.nvim_feedkeys(vim.api.nvim_replace_termcodes("<BS>", true, false, true), "n", false)
    return
  end

  M._update_display()
end

--- Check if input mode is enabled
--- @return boolean
function M.is_enabled()
  return M.state.enabled
end

return M
