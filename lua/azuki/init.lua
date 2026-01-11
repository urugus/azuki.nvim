--- azuki.nvim - Japanese input plugin for Neovim
--- Phase 2a: Basic input functionality

local M = {}

local config = require("azuki.config")
local server = require("azuki.server")
local input = require("azuki.input")
local ui = require("azuki.ui")

--- Setup highlight groups
local function setup_highlights()
  local hl = config.get("highlight")
  vim.api.nvim_set_hl(0, hl.pending, { underline = true, default = true })
  vim.api.nvim_set_hl(0, hl.selected, { reverse = true, default = true })
  vim.api.nvim_set_hl(0, hl.segment, { underline = true, default = true })
  vim.api.nvim_set_hl(0, hl.current_segment, { reverse = true, bold = true, default = true })
end

--- Setup user commands
local function setup_commands()
  vim.api.nvim_create_user_command("AzukiStart", function()
    M.start()
  end, { desc = "Start azuki server" })

  vim.api.nvim_create_user_command("AzukiStop", function()
    M.stop()
  end, { desc = "Stop azuki server" })

  vim.api.nvim_create_user_command("AzukiStatus", function()
    M.status()
  end, { desc = "Show azuki status" })

  vim.api.nvim_create_user_command("AzukiToggle", function()
    M.toggle()
  end, { desc = "Toggle Japanese input mode" })

  vim.api.nvim_create_user_command("AzukiTest", function(cmd)
    M.test_convert(cmd.args)
  end, { desc = "Test conversion", nargs = "?" })
end

--- Setup toggle key mapping
local function setup_toggle_key()
  local toggle_key = config.get("toggle_key")
  vim.keymap.set("i", toggle_key, function()
    input.toggle()
  end, { noremap = true, desc = "Toggle azuki Japanese input" })
end

--- Setup function
--- @param opts table|nil User configuration
function M.setup(opts)
  -- Initialize configuration
  config.setup(opts)

  -- Initialize UI module
  ui.setup()

  -- Setup highlight groups
  setup_highlights()

  -- Setup user commands
  setup_commands()

  -- Setup toggle key
  setup_toggle_key()

  -- Auto-stop server on Neovim exit
  vim.api.nvim_create_autocmd("VimLeavePre", {
    callback = function()
      if server.is_active() then
        server.stop()
      end
    end,
  })

  local toggle_key = config.get("toggle_key")
  vim.notify(
    "[azuki] Plugin loaded. Press " .. toggle_key .. " in Insert mode to enable Japanese input.",
    vim.log.levels.INFO
  )
end

--- Start the server
function M.start()
  server.start({ server_path = config.get("server_path") }, function(success)
    if success then
      vim.notify("[azuki] Server started successfully", vim.log.levels.INFO)
    end
  end)
end

--- Stop the server
function M.stop()
  server.stop(function()
    vim.notify("[azuki] Server stopped", vim.log.levels.INFO)
  end)
end

--- Show status
function M.status()
  local server_status = server.is_active() and "running" or "stopped"
  local input_status = input.is_enabled() and "enabled" or "disabled"
  vim.notify(string.format("[azuki] Server: %s, Input: %s", server_status, input_status), vim.log.levels.INFO)
end

--- Test conversion (for verification)
--- @param reading string|nil Test input (hiragana)
function M.test_convert(reading)
  if not server.is_active() then
    vim.notify("[azuki] Server not running. Use :AzukiStart first.", vim.log.levels.WARN)
    return
  end

  reading = reading or "きょうは"
  vim.notify("[azuki] Testing conversion: " .. reading, vim.log.levels.INFO)

  server.convert(reading, { live = true }, function(response)
    if response.type == "convert_result" then
      local candidates = table.concat(response.candidates, ", ")
      vim.notify("[azuki] Candidates: " .. candidates, vim.log.levels.INFO)
    elseif response.type == "error" then
      vim.notify("[azuki] Error: " .. response.error, vim.log.levels.ERROR)
    end
  end)
end

--- Public API: Enable Japanese input mode
M.enable = input.enable

--- Public API: Disable Japanese input mode
M.disable = input.disable

--- Public API: Toggle Japanese input mode
M.toggle = input.toggle

--- Public API: Check if input mode is enabled
M.is_enabled = input.is_enabled

return M
