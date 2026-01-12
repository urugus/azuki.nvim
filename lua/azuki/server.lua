--- azuki-server communication module
--- Handles spawning and communicating with the azuki-server process

local M = {}

--- Server state
M.handle = nil
M.stdin = nil
M.stdout = nil
M.stderr = nil
M.seq = 0
M.session_id = nil
M.callbacks = {}
M.read_buffer = ""
M.is_running = false
M.stop_callback = nil -- Callback to invoke after server exit

--- Configuration
local config = {
  server_path = nil, -- Will be set during initialization
}

--- Find the server binary path
--- @return string|nil path to the server binary, or nil if not found
local function find_server_path()
  -- Check explicit config first
  if config.server_path and vim.fn.filereadable(config.server_path) == 1 then
    return config.server_path
  end

  -- Check default location
  local default_path = vim.fn.stdpath("data") .. "/azuki/bin/azuki-server"
  if vim.fn.filereadable(default_path) == 1 then
    return default_path
  end

  -- Check if we're in development mode (server binary in project directory)
  local plugin_root = vim.fn.fnamemodify(debug.getinfo(1, "S").source:sub(2), ":h:h:h")
  local dev_path = plugin_root .. "/server/target/debug/azuki-server"
  if vim.fn.filereadable(dev_path) == 1 then
    return dev_path
  end

  local release_path = plugin_root .. "/server/target/release/azuki-server"
  if vim.fn.filereadable(release_path) == 1 then
    return release_path
  end

  return nil
end

--- Pack a 32-bit big-endian unsigned integer
--- @param n number
--- @return string
local function pack_u32_be(n)
  return string.char(
    bit.band(bit.rshift(n, 24), 0xFF),
    bit.band(bit.rshift(n, 16), 0xFF),
    bit.band(bit.rshift(n, 8), 0xFF),
    bit.band(n, 0xFF)
  )
end

--- Unpack a 32-bit big-endian unsigned integer
--- @param s string
--- @return number
local function unpack_u32_be(s)
  local b1, b2, b3, b4 = s:byte(1, 4)
  return bit.lshift(b1, 24) + bit.lshift(b2, 16) + bit.lshift(b3, 8) + b4
end

--- Send a message to the server
--- @param msg table
--- @param callback function|nil
function M.send(msg, callback)
  if not M.is_running or not M.stdin then
    vim.notify("[azuki] Server not running", vim.log.levels.ERROR)
    return
  end

  M.seq = M.seq + 1
  msg.seq = M.seq

  if M.session_id then
    msg.session_id = msg.session_id or M.session_id
  end

  if callback then
    M.callbacks[M.seq] = callback
  end

  local json = vim.fn.json_encode(msg)
  local len = #json
  local frame = pack_u32_be(len) .. json

  M.stdin:write(frame)
end

--- Process received data from server
--- @param data string
local function process_data(data)
  M.read_buffer = M.read_buffer .. data

  while true do
    -- Need at least 4 bytes for length prefix
    if #M.read_buffer < 4 then
      return
    end

    local len = unpack_u32_be(M.read_buffer:sub(1, 4))

    -- Check if we have the full message
    if #M.read_buffer < 4 + len then
      return
    end

    -- Extract the message
    local json_str = M.read_buffer:sub(5, 4 + len)
    M.read_buffer = M.read_buffer:sub(5 + len)

    -- Parse and handle the response
    local ok, response = pcall(vim.fn.json_decode, json_str)
    if ok and response then
      -- Handle init response specially to store session_id
      if response.type == "init_result" and response.session_id then
        M.session_id = response.session_id
      end

      -- Call registered callback
      local seq = response.seq
      if seq and M.callbacks[seq] then
        vim.schedule(function()
          M.callbacks[seq](response)
          M.callbacks[seq] = nil
        end)
      end
    else
      vim.notify("[azuki] Failed to parse server response: " .. json_str, vim.log.levels.WARN)
    end
  end
end

--- Start the server process
--- @param opts table|nil Optional configuration
--- @param callback function|nil Called when server is initialized
function M.start(opts, callback)
  if M.is_running then
    if callback then
      callback(true)
    end
    return
  end

  opts = opts or {}
  if opts.server_path then
    config.server_path = opts.server_path
  end

  local server_path = find_server_path()
  if not server_path then
    vim.notify("[azuki] azuki-server not found. Please build it or set server_path.", vim.log.levels.ERROR)
    if callback then
      callback(false)
    end
    return
  end

  local stdin = vim.uv.new_pipe(false)
  local stdout = vim.uv.new_pipe(false)
  local stderr = vim.uv.new_pipe(false)

  local handle, pid
  handle, pid = vim.uv.spawn(server_path, {
    stdio = { stdin, stdout, stderr },
  }, function(code, signal)
    vim.schedule(function()
      M.is_running = false
      M.handle = nil
      M.stdin = nil
      M.stdout = nil
      M.stderr = nil
      M.session_id = nil

      if code ~= 0 then
        vim.notify("[azuki] Server exited with code " .. code, vim.log.levels.WARN)
      end

      -- Invoke stop callback after cleanup is complete
      if M.stop_callback then
        local cb = M.stop_callback
        M.stop_callback = nil
        cb()
      end
    end)
  end)

  if not handle then
    vim.notify("[azuki] Failed to spawn server: " .. tostring(pid), vim.log.levels.ERROR)
    stdin:close()
    stdout:close()
    stderr:close()
    if callback then
      callback(false)
    end
    return
  end

  M.handle = handle
  M.stdin = stdin
  M.stdout = stdout
  M.stderr = stderr
  M.is_running = true
  M.read_buffer = ""
  M.seq = 0
  M.callbacks = {}

  -- Read stdout
  stdout:read_start(function(err, data)
    if err then
      vim.schedule(function()
        vim.notify("[azuki] Read error: " .. err, vim.log.levels.ERROR)
      end)
      return
    end

    if data then
      -- Schedule to main thread since we need to call vim.fn
      vim.schedule(function()
        process_data(data)
      end)
    end
  end)

  -- Read stderr (for debug messages)
  stderr:read_start(function(err, data)
    if data then
      vim.schedule(function()
        vim.notify("[azuki-server] " .. data:gsub("\n$", ""), vim.log.levels.DEBUG)
      end)
    end
  end)

  -- Send init message with zenzai config if enabled
  local init_msg = { type = "init" }

  -- Include zenzai configuration if available
  local azuki_config = require("azuki.config")
  local zenzai_config = azuki_config.get("zenzai")
  if zenzai_config and zenzai_config.enabled then
    init_msg.zenzai = zenzai_config
  end

  M.send(init_msg, function(response)
    if response.type == "init_result" then
      local info_parts = { "[azuki] Server initialized (v" .. response.version .. ")" }
      if response.zenzai_enabled then
        table.insert(info_parts, " with Zenzai")
      end
      vim.notify(table.concat(info_parts), vim.log.levels.INFO)
      if callback then
        callback(true)
      end
    else
      vim.notify("[azuki] Server init failed", vim.log.levels.ERROR)
      if callback then
        callback(false)
      end
    end
  end)
end

--- Stop the server process
--- @param callback function|nil Called when server is stopped and cleanup is complete
function M.stop(callback)
  if not M.is_running then
    if callback then
      callback()
    end
    return
  end

  -- Store callback to be invoked after exit cleanup
  M.stop_callback = callback

  M.send({ type = "shutdown" }, function()
    -- Callback will be invoked by exit handler after cleanup
  end)
end

--- Send a convert request
--- @param reading string Hiragana string to convert
--- @param opts table|nil Options (cursor, live, etc.)
--- @param callback function Called with response
function M.convert(reading, opts, callback)
  if not M.session_id then
    vim.notify("[azuki] Server not initialized yet", vim.log.levels.WARN)
    if callback then
      callback({ type = "error", error = "Server not initialized" })
    end
    return
  end

  opts = opts or {}
  M.send({
    type = "convert",
    reading = reading,
    cursor = opts.cursor,
    options = {
      live = opts.live or false,
    },
  }, callback)
end

--- Send a commit request
--- @param reading string Original hiragana
--- @param candidate string Selected candidate
--- @param callback function|nil Called with response
function M.commit(reading, candidate, callback)
  if not M.session_id then
    vim.notify("[azuki] Server not initialized yet", vim.log.levels.WARN)
    if callback then
      callback({ type = "error", error = "Server not initialized" })
    end
    return
  end

  M.send({
    type = "commit",
    reading = reading,
    candidate = candidate,
  }, callback)
end

--- Send an adjust_segment request
--- @param reading string Full hiragana reading
--- @param segments table[] Current segment information
--- @param segment_index number Segment index (0-indexed for server)
--- @param direction string "shrink" or "extend"
--- @param callback function Called with response
function M.adjust_segment(reading, segments, segment_index, direction, callback)
  if not M.session_id then
    vim.notify("[azuki] Server not initialized yet", vim.log.levels.WARN)
    if callback then
      callback({ type = "error", error = "Server not initialized" })
    end
    return
  end

  M.send({
    type = "adjust_segment",
    reading = reading,
    segments = segments,
    segment_index = segment_index,
    direction = direction,
  }, callback)
end

--- Check if server is running
--- @return boolean
function M.is_active()
  return M.is_running
end

--- Get current sequence number
--- @return number
function M.get_seq()
  return M.seq
end

return M
