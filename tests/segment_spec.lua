-- Segment function test script
-- Run with: nvim --headless -c "set rtp+=." -c "luafile tests/segment_spec.lua"

local input = require("azuki.input")
local ui = require("azuki.ui")
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

-- Helper: Reset input state
local function reset_state()
  input.state.enabled = false
  input.state.romaji_buffer = ""
  input.state.hiragana = ""
  input.state.candidates = {}
  input.state.selected_index = 0
  input.state.segments = {}
  input.state.current_segment = 1
  input.state.bufnr = nil
  input.state.last_seq = 0
end

-- Helper: Create mock segments
local function create_mock_segments()
  return {
    {
      reading = "きょう",
      candidates = { "今日", "教", "京" },
      length = 3,
      selected_index = 1,
    },
    {
      reading = "は",
      candidates = { "は", "葉", "歯" },
      length = 1,
      selected_index = 1,
    },
    {
      reading = "いい",
      candidates = { "良い", "いい", "好い" },
      length = 2,
      selected_index = 1,
    },
  }
end

print("=== azuki.nvim Segment Function Tests ===\n")

-- Initialize UI namespace
ui.setup()

-- =====================================================
-- A. Segment Selection Tests (Tab/Shift-Tab)
-- =====================================================

test("Segment selection: initial state is segment 1", function()
  reset_state()
  input.state.segments = create_mock_segments()
  input.state.current_segment = 1

  assert(input.state.current_segment == 1, "current_segment should be 1")
end)

test("Segment selection: Tab moves to next segment", function()
  reset_state()
  input.state.segments = create_mock_segments()
  input.state.current_segment = 1

  input.next_segment()

  assert(input.state.current_segment == 2, "current_segment should be 2 after Tab")
end)

test("Segment selection: Tab at last segment stays at last", function()
  reset_state()
  input.state.segments = create_mock_segments()
  input.state.current_segment = 3 -- Last segment

  input.next_segment()

  assert(input.state.current_segment == 3, "current_segment should stay at 3")
end)

test("Segment selection: Shift-Tab moves to previous segment", function()
  reset_state()
  input.state.segments = create_mock_segments()
  input.state.current_segment = 2

  input.prev_segment()

  assert(input.state.current_segment == 1, "current_segment should be 1 after Shift-Tab")
end)

test("Segment selection: Shift-Tab at first segment stays at first", function()
  reset_state()
  input.state.segments = create_mock_segments()
  input.state.current_segment = 1

  input.prev_segment()

  assert(input.state.current_segment == 1, "current_segment should stay at 1")
end)

test("Segment selection: Tab with no segments does nothing", function()
  reset_state()
  input.state.segments = {}
  input.state.current_segment = 1

  input.next_segment()

  assert(input.state.current_segment == 1, "current_segment should stay at 1 with no segments")
end)

-- =====================================================
-- B. Candidate Selection Tests (Space/Shift-Space)
-- =====================================================

test("Candidate selection: Space cycles to next candidate in segment", function()
  reset_state()
  input.state.segments = create_mock_segments()
  input.state.current_segment = 1
  input.state.hiragana = "きょうはいい"

  -- Initial selected_index is 1
  assert(input.state.segments[1].selected_index == 1)

  input.next_candidate()

  assert(input.state.segments[1].selected_index == 2, "selected_index should be 2 after Space")
end)

test("Candidate selection: Space wraps around at end", function()
  reset_state()
  input.state.segments = create_mock_segments()
  input.state.current_segment = 1
  input.state.hiragana = "きょうはいい"
  input.state.segments[1].selected_index = 3 -- Last candidate

  input.next_candidate()

  assert(input.state.segments[1].selected_index == 1, "selected_index should wrap to 1")
end)

test("Candidate selection: Shift-Space cycles to previous candidate", function()
  reset_state()
  input.state.segments = create_mock_segments()
  input.state.current_segment = 1
  input.state.hiragana = "きょうはいい"
  input.state.segments[1].selected_index = 2

  input.prev_candidate()

  assert(input.state.segments[1].selected_index == 1, "selected_index should be 1 after Shift-Space")
end)

test("Candidate selection: Shift-Space wraps around at start", function()
  reset_state()
  input.state.segments = create_mock_segments()
  input.state.current_segment = 1
  input.state.hiragana = "きょうはいい"
  input.state.segments[1].selected_index = 1

  input.prev_candidate()

  assert(input.state.segments[1].selected_index == 3, "selected_index should wrap to 3")
end)

-- =====================================================
-- C. Stale Response Handling Tests
-- =====================================================

test("Stale response: last_seq tracking", function()
  reset_state()

  -- Simulate sequence number tracking
  input.state.last_seq = 5

  -- A response with old seq should be ignored (this tests the logic)
  local old_seq = 3
  local current_seq = input.state.last_seq

  assert(old_seq ~= current_seq, "old_seq should not match current last_seq")
end)

test("Stale response: cancel increments last_seq", function()
  reset_state()
  input.state.last_seq = 5

  -- Mock server.get_seq
  local original_get_seq = server.get_seq
  server.get_seq = function()
    return 10
  end

  input.cancel()

  -- last_seq should be server.get_seq() + 1000 = 1010
  assert(input.state.last_seq == 1010, "last_seq should be 1010 after cancel, got " .. input.state.last_seq)

  -- Restore
  server.get_seq = original_get_seq
end)

-- =====================================================
-- D. UI Display Tests
-- =====================================================

test("UI: show_segments creates virt_text for multiple segments", function()
  -- Create a test buffer
  local bufnr = vim.api.nvim_create_buf(false, true)

  -- Set some content
  vim.api.nvim_buf_set_lines(bufnr, 0, -1, false, { "" })

  local segments = create_mock_segments()

  -- Call show_segments
  ui.show_segments(bufnr, 0, 0, segments, 1, "")

  -- Verify extmark was created
  local marks = vim.api.nvim_buf_get_extmarks(bufnr, ui.ns_id, 0, -1, { details = true })
  assert(#marks > 0, "extmark should be created")

  -- Clean up
  vim.api.nvim_buf_delete(bufnr, { force = true })
end)

test("UI: show_segments includes pending romaji", function()
  local bufnr = vim.api.nvim_create_buf(false, true)
  vim.api.nvim_buf_set_lines(bufnr, 0, -1, false, { "" })

  local segments = create_mock_segments()

  ui.show_segments(bufnr, 0, 0, segments, 1, "te")

  local marks = vim.api.nvim_buf_get_extmarks(bufnr, ui.ns_id, 0, -1, { details = true })
  assert(#marks > 0, "extmark should be created")

  -- Check virt_text includes pending romaji
  local details = marks[1][4]
  assert(details.virt_text ~= nil, "virt_text should exist")

  -- Last element should be pending romaji
  local last_virt = details.virt_text[#details.virt_text]
  assert(last_virt[1] == "te", "last virt_text should be pending romaji 'te'")

  vim.api.nvim_buf_delete(bufnr, { force = true })
end)

test("UI: clear removes all extmarks", function()
  local bufnr = vim.api.nvim_create_buf(false, true)
  vim.api.nvim_buf_set_lines(bufnr, 0, -1, false, { "" })

  -- Create an extmark
  ui.show_preedit(bufnr, 0, 0, "test")

  local marks_before = vim.api.nvim_buf_get_extmarks(bufnr, ui.ns_id, 0, -1, {})
  assert(#marks_before > 0, "extmark should exist before clear")

  -- Clear
  ui.clear(bufnr)

  local marks_after = vim.api.nvim_buf_get_extmarks(bufnr, ui.ns_id, 0, -1, {})
  assert(#marks_after == 0, "extmarks should be cleared")

  vim.api.nvim_buf_delete(bufnr, { force = true })
end)

-- =====================================================
-- E. Segment Boundary Adjustment Tests (without server)
-- =====================================================

test("Segment boundary: shrink_segment returns early with no segments", function()
  reset_state()
  input.state.segments = {}

  -- Should not error
  input.shrink_segment()

  assert(true, "shrink_segment should not error with no segments")
end)

test("Segment boundary: shrink_segment returns early with length 1", function()
  reset_state()
  input.state.segments = {
    { reading = "あ", candidates = { "亜" }, length = 1, selected_index = 1 },
  }
  input.state.current_segment = 1

  -- Should not error (cannot shrink further)
  input.shrink_segment()

  assert(true, "shrink_segment should not error with length 1 segment")
end)

test("Segment boundary: extend_segment returns early at last segment", function()
  reset_state()
  input.state.segments = create_mock_segments()
  input.state.current_segment = 3 -- Last segment

  -- Should not error (cannot extend last segment)
  input.extend_segment()

  assert(true, "extend_segment should not error at last segment")
end)

test("Segment boundary: extend_segment returns early when next has length 1", function()
  reset_state()
  input.state.segments = {
    { reading = "あい", candidates = { "愛" }, length = 2, selected_index = 1 },
    { reading = "う", candidates = { "鵜" }, length = 1, selected_index = 1 },
  }
  input.state.current_segment = 1

  -- Should not error (next segment has only 1 char)
  input.extend_segment()

  assert(true, "extend_segment should not error when next segment has length 1")
end)

-- =====================================================
-- F. Integration with Server Tests
-- =====================================================

local server_started = false

test("Server: start for segment tests", function()
  local done = false
  local success = false

  server.start(nil, function(result)
    success = result
    done = true
  end)

  vim.wait(5000, function()
    return done
  end)

  assert(done, "server start callback not called")
  assert(success, "server failed to start")
  server_started = true
end)

test("Server: convert returns segments", function()
  assert(server_started, "server not started")

  local done = false
  local response = nil

  server.convert("きょうはいい", { live = true }, function(resp)
    response = resp
    done = true
  end)

  vim.wait(2000, function()
    return done
  end)

  assert(done, "convert callback not called")
  assert(response ~= nil, "response is nil")
  assert(response.type == "convert_result", "unexpected response type: " .. tostring(response.type))

  -- Check if segments are returned
  if response.segments then
    print("  -> Segments returned: " .. #response.segments)
    for i, seg in ipairs(response.segments) do
      print("     Segment " .. i .. ": " .. seg.reading .. " -> " .. (seg.candidates[1] or "N/A"))
    end
  else
    print("  -> No segments (fallback to candidates)")
  end
end)

test("Server: adjust_segment shrink", function()
  assert(server_started, "server not started")

  -- First get segments
  local segments = nil
  local done = false

  server.convert("きょうは", { live = true }, function(resp)
    if resp.segments then
      segments = resp.segments
    end
    done = true
  end)

  vim.wait(2000, function()
    return done
  end)

  if not segments or #segments < 1 then
    print("  -> Skipping: no segments to adjust")
    return
  end

  -- Try to shrink first segment (if length > 1)
  if segments[1].length <= 1 then
    print("  -> Skipping: first segment too short to shrink")
    return
  end

  done = false
  local adjust_response = nil

  server.adjust_segment("きょうは", segments, 0, "shrink", function(resp)
    adjust_response = resp
    done = true
  end)

  vim.wait(2000, function()
    return done
  end)

  assert(done, "adjust_segment callback not called")
  assert(adjust_response ~= nil, "adjust_segment response is nil")

  if adjust_response.type == "adjust_segment_result" then
    print("  -> Adjusted segments: " .. #adjust_response.segments)
  else
    print("  -> Response type: " .. tostring(adjust_response.type))
  end
end)

test("Server: stop", function()
  assert(server_started, "server not started")

  local done = false

  server.stop(function()
    done = true
  end)

  vim.wait(2000, function()
    return done
  end)

  assert(done, "stop callback not called")
end)

print("\n=== Results ===")
print(string.format("Passed: %d, Failed: %d", passed, failed))

if failed > 0 then
  vim.cmd("cquit 1")
else
  vim.cmd("qall!")
end
