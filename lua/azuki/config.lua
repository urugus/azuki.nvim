--- azuki.nvim configuration module
--- Centralized configuration management

local M = {}

--- Default configuration values
M.defaults = {
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

--- Current configuration values
M.values = vim.deepcopy(M.defaults)

--- Setup configuration with user options
--- @param opts table|nil User configuration
function M.setup(opts)
  M.values = vim.tbl_deep_extend("force", M.defaults, opts or {})
end

--- Get a configuration value
--- @param key string Configuration key
--- @return any
function M.get(key)
  return M.values[key]
end

--- Get the entire configuration
--- @return table
function M.all()
  return M.values
end

return M
