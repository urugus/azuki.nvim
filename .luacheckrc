-- vim: ft=lua

std = "luajit"

-- Neovim globals
read_globals = {
  "vim",
  "bit",
}

globals = {
  "vim",
}

-- Exclude files
exclude_files = {
  ".luarocks",
  ".install",
}

-- Max line length
max_line_length = 120

-- Ignore specific warnings
ignore = {
  "212", -- Unused argument
}

-- Cache results
cache = true
