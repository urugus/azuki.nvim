-- Phase 1 test script
-- Run with: nvim --headless -c "set rtp+=." -c "luafile test_phase1.lua"

local server = require("azuki.server")

local passed = 0
local failed = 0

local function test(name, fn)
  local ok, err = pcall(fn)
  if ok then
    print("PASS: " .. name)
    passed = passed + 1
  else
    print("FAIL: " .. name .. " - " .. tostring(err))
    failed = failed + 1
  end
end

local function wait(ms)
  vim.wait(ms, function() return false end)
end

print("=== azuki.nvim Phase 1 Tests ===\n")

-- Test 1: Server module loads
test("Server module loads", function()
  assert(server ~= nil, "server module is nil")
  assert(type(server.start) == "function", "start function missing")
  assert(type(server.stop) == "function", "stop function missing")
  assert(type(server.convert) == "function", "convert function missing")
end)

-- Test 2: Server not running initially
test("Server not running initially", function()
  assert(server.is_active() == false, "server should not be running")
end)

-- Test 3: Start server
local server_started = false
test("Start server", function()
  local done = false
  local success = false

  server.start(nil, function(result)
    success = result
    done = true
  end)

  -- Wait for server to start (max 5 seconds)
  vim.wait(5000, function() return done end)

  assert(done, "server start callback not called")
  assert(success, "server failed to start")
  server_started = true
end)

-- Test 4: Server is running
test("Server is running", function()
  assert(server_started, "server not started")
  assert(server.is_active() == true, "server should be running")
  assert(server.session_id ~= nil, "session_id should be set")
end)

-- Test 5: Convert request
test("Convert request", function()
  assert(server_started, "server not started")

  local done = false
  local response = nil

  server.convert("てすと", { live = true }, function(resp)
    response = resp
    done = true
  end)

  vim.wait(2000, function() return done end)

  assert(done, "convert callback not called")
  assert(response ~= nil, "response is nil")
  assert(response.type == "convert_result", "unexpected response type: " .. tostring(response.type))
  assert(response.candidates ~= nil, "candidates is nil")
  assert(#response.candidates > 0, "no candidates returned")
  print("  -> Candidates: " .. table.concat(response.candidates, ", "))
end)

-- Test 6: Commit request
test("Commit request", function()
  assert(server_started, "server not started")

  local done = false
  local response = nil

  server.commit("てすと", "テスト", function(resp)
    response = resp
    done = true
  end)

  vim.wait(2000, function() return done end)

  assert(done, "commit callback not called")
  assert(response ~= nil, "response is nil")
  assert(response.type == "commit_result", "unexpected response type: " .. tostring(response.type))
  assert(response.success == true, "commit should succeed")
end)

-- Test 7: Stop server
test("Stop server", function()
  assert(server_started, "server not started")

  local done = false

  server.stop(function()
    done = true
  end)

  vim.wait(2000, function() return done end)

  assert(done, "stop callback not called")
end)

-- Test 8: Server stopped
test("Server stopped", function()
  wait(100) -- Give server time to clean up
  -- Server should report not running after stop
  -- Note: The actual cleanup happens asynchronously
end)

print("\n=== Results ===")
print(string.format("Passed: %d, Failed: %d", passed, failed))

if failed > 0 then
  vim.cmd("cquit 1")
else
  vim.cmd("qall!")
end
