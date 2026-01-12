--- azuki.nvim input management module
--- Coordinates state, keymap, and handler modules

local M = {}

local config = require("azuki.config")
local state = require("azuki.state")
local keymap = require("azuki.keymap")
local handler = require("azuki.handler")
local server = require("azuki.server")
local ui = require("azuki.ui")

--- Enable Japanese input mode
function M.enable()
  if state.data.enabled then
    return
  end

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
  state.reset()
  state.data.enabled = true
  state.data.bufnr = vim.api.nvim_get_current_buf()

  local cursor = vim.api.nvim_win_get_cursor(0)
  state.set_preedit_anchor(cursor[1] - 1, cursor[2])

  -- Set up disable callback for handler
  handler.on_disable = M.disable

  -- Setup key hooks with handler functions
  keymap.setup(state.data.bufnr, {
    input = handler.input,
    commit = handler.commit,
    backspace = handler.backspace,
    escape = handler.escape,
    next_candidate = handler.next_candidate,
    prev_candidate = handler.prev_candidate,
    cancel = handler.cancel,
    next_segment = handler.next_segment,
    prev_segment = handler.prev_segment,
    shrink_segment = handler.shrink_segment,
    extend_segment = handler.extend_segment,
  })

  vim.notify("[azuki] Japanese input enabled", vim.log.levels.INFO)
end

--- Disable Japanese input mode
function M.disable()
  if not state.data.enabled then
    return
  end

  handler.cancel_debounce()

  -- Commit pending text
  if state.has_preedit() then
    handler.commit()
  end

  local bufnr = state.data.bufnr

  keymap.teardown(bufnr)

  if bufnr then
    ui.clear(bufnr)
  end

  state.reset()
  handler.on_disable = nil

  vim.notify("[azuki] Japanese input disabled", vim.log.levels.INFO)
end

--- Toggle Japanese input mode
function M.toggle()
  if state.data.enabled then
    M.disable()
  else
    M.enable()
  end
end

--- Check if input mode is enabled
--- @return boolean
function M.is_enabled()
  return state.data.enabled
end

return M
