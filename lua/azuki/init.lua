--- azuki.nvim - Japanese input plugin for Neovim
--- Phase 1: Server startup and communication test

local M = {}

local server = require("azuki.server")

--- Default configuration
M.config = {
  server_path = nil, -- Auto-detect or explicit path
  debounce_ms = 30,
  toggle_key = "<C-j>",
  live_conversion = true,
  highlight = {
    pending = "AzukiPending",
    selected = "AzukiSelected",
  },
  learning = true,
  learning_file = vim.fn.stdpath("data") .. "/azuki/learning.json",
}

--- Setup highlight groups
local function setup_highlights()
  vim.api.nvim_set_hl(0, "AzukiPending", { underline = true, default = true })
  vim.api.nvim_set_hl(0, "AzukiSelected", { reverse = true, default = true })
end

--- Setup function
--- @param opts table|nil User configuration
function M.setup(opts)
  opts = opts or {}
  M.config = vim.tbl_deep_extend("force", M.config, opts)

  setup_highlights()

  -- Create user commands for Phase 1 testing
  vim.api.nvim_create_user_command("AzukiStart", function()
    M.start()
  end, { desc = "Start azuki server" })

  vim.api.nvim_create_user_command("AzukiStop", function()
    M.stop()
  end, { desc = "Stop azuki server" })

  vim.api.nvim_create_user_command("AzukiStatus", function()
    M.status()
  end, { desc = "Show azuki server status" })

  vim.api.nvim_create_user_command("AzukiTest", function(cmd)
    M.test_convert(cmd.args)
  end, { desc = "Test conversion", nargs = "?" })

  -- Auto-stop server on Neovim exit
  vim.api.nvim_create_autocmd("VimLeavePre", {
    callback = function()
      if server.is_active() then
        server.stop()
      end
    end,
  })

  vim.notify("[azuki] Plugin loaded. Use :AzukiStart to start the server.", vim.log.levels.INFO)
end

--- Start the server
function M.start()
  server.start({ server_path = M.config.server_path }, function(success)
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

--- Show server status
function M.status()
  if server.is_active() then
    vim.notify("[azuki] Server is running (session: " .. (server.session_id or "unknown") .. ")", vim.log.levels.INFO)
  else
    vim.notify("[azuki] Server is not running", vim.log.levels.INFO)
  end
end

--- Test conversion (for Phase 1 verification)
--- @param input string|nil Test input (hiragana)
function M.test_convert(input)
  if not server.is_active() then
    vim.notify("[azuki] Server not running. Use :AzukiStart first.", vim.log.levels.WARN)
    return
  end

  input = input or "きょうは"
  vim.notify("[azuki] Testing conversion: " .. input, vim.log.levels.INFO)

  server.convert(input, { live = true }, function(response)
    if response.type == "convert_result" then
      local candidates = table.concat(response.candidates, ", ")
      vim.notify("[azuki] Candidates: " .. candidates, vim.log.levels.INFO)
    elseif response.type == "error" then
      vim.notify("[azuki] Error: " .. response.error, vim.log.levels.ERROR)
    end
  end)
end

return M
