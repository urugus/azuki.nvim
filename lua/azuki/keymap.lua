--- azuki.nvim keymap module
--- Table-driven key mapping management

local M = {}

--- Control key definitions
--- Maps key to action name
M.control_keys = {
  ["<CR>"] = "commit",
  ["<BS>"] = "backspace",
  ["<Esc>"] = "escape",
  ["<Space>"] = "next_candidate",
  ["<S-Space>"] = "prev_candidate",
  ["<C-g>"] = "cancel",
  ["<Tab>"] = "next_segment",
  ["<S-Tab>"] = "prev_segment",
  ["<S-Left>"] = "shrink_segment",
  ["<S-Right>"] = "extend_segment",
}

--- Special character keys that should be handled as input
M.special_keys = { "-", "'" }

--- Setup key mappings for a buffer
--- @param bufnr number Buffer number
--- @param handlers table Table of handler functions keyed by action name
function M.setup(bufnr, handlers)
  -- Alphabet keys (a-z)
  for c = string.byte("a"), string.byte("z") do
    local key = string.char(c)
    vim.keymap.set("i", key, function()
      handlers.input(key)
    end, { buffer = bufnr, noremap = true })
  end

  -- Uppercase keys (A-Z)
  for c = string.byte("A"), string.byte("Z") do
    local key = string.char(c)
    vim.keymap.set("i", key, function()
      handlers.input(key)
    end, { buffer = bufnr, noremap = true })
  end

  -- Special character keys
  for _, key in ipairs(M.special_keys) do
    vim.keymap.set("i", key, function()
      handlers.input(key)
    end, { buffer = bufnr, noremap = true })
  end

  -- Control keys
  for key, action in pairs(M.control_keys) do
    if handlers[action] then
      vim.keymap.set("i", key, handlers[action], { buffer = bufnr, noremap = true })
    end
  end
end

--- Teardown key mappings for a buffer
--- @param bufnr number Buffer number
function M.teardown(bufnr)
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
  for _, key in ipairs(M.special_keys) do
    pcall(vim.keymap.del, "i", key, { buffer = bufnr })
  end

  -- Remove control key mappings
  for key, _ in pairs(M.control_keys) do
    pcall(vim.keymap.del, "i", key, { buffer = bufnr })
  end
end

return M
