--- azuki.nvim romaji to hiragana conversion module
--- Converts romaji input to hiragana using longest-match algorithm

local M = {}

--- Romaji to hiragana conversion table
--- Keys are sorted by length (longest first) for matching
M.table = {
  -- 4-character patterns
  ["ltsu"] = "っ",
  ["xtsu"] = "っ",

  -- 3-character patterns
  ["kya"] = "きゃ",
  ["kyi"] = "きぃ",
  ["kyu"] = "きゅ",
  ["kye"] = "きぇ",
  ["kyo"] = "きょ",
  ["sha"] = "しゃ",
  ["shi"] = "し",
  ["shu"] = "しゅ",
  ["she"] = "しぇ",
  ["sho"] = "しょ",
  ["sya"] = "しゃ",
  ["syi"] = "しぃ",
  ["syu"] = "しゅ",
  ["sye"] = "しぇ",
  ["syo"] = "しょ",
  ["cha"] = "ちゃ",
  ["chi"] = "ち",
  ["chu"] = "ちゅ",
  ["che"] = "ちぇ",
  ["cho"] = "ちょ",
  ["tya"] = "ちゃ",
  ["tyi"] = "ちぃ",
  ["tyu"] = "ちゅ",
  ["tye"] = "ちぇ",
  ["tyo"] = "ちょ",
  ["tha"] = "てゃ",
  ["thi"] = "てぃ",
  ["thu"] = "てゅ",
  ["the"] = "てぇ",
  ["tho"] = "てょ",
  ["tsu"] = "つ",
  ["nya"] = "にゃ",
  ["nyi"] = "にぃ",
  ["nyu"] = "にゅ",
  ["nye"] = "にぇ",
  ["nyo"] = "にょ",
  ["hya"] = "ひゃ",
  ["hyi"] = "ひぃ",
  ["hyu"] = "ひゅ",
  ["hye"] = "ひぇ",
  ["hyo"] = "ひょ",
  ["mya"] = "みゃ",
  ["myi"] = "みぃ",
  ["myu"] = "みゅ",
  ["mye"] = "みぇ",
  ["myo"] = "みょ",
  ["rya"] = "りゃ",
  ["ryi"] = "りぃ",
  ["ryu"] = "りゅ",
  ["rye"] = "りぇ",
  ["ryo"] = "りょ",
  ["gya"] = "ぎゃ",
  ["gyi"] = "ぎぃ",
  ["gyu"] = "ぎゅ",
  ["gye"] = "ぎぇ",
  ["gyo"] = "ぎょ",
  ["jya"] = "じゃ",
  ["jyi"] = "じぃ",
  ["jyu"] = "じゅ",
  ["jye"] = "じぇ",
  ["jyo"] = "じょ",
  ["bya"] = "びゃ",
  ["byi"] = "びぃ",
  ["byu"] = "びゅ",
  ["bye"] = "びぇ",
  ["byo"] = "びょ",
  ["pya"] = "ぴゃ",
  ["pyi"] = "ぴぃ",
  ["pyu"] = "ぴゅ",
  ["pye"] = "ぴぇ",
  ["pyo"] = "ぴょ",
  ["xya"] = "ゃ",
  ["xyu"] = "ゅ",
  ["xyo"] = "ょ",
  ["lya"] = "ゃ",
  ["lyu"] = "ゅ",
  ["lyo"] = "ょ",
  ["xtu"] = "っ",
  ["ltu"] = "っ",
  ["xwa"] = "ゎ",
  ["lwa"] = "ゎ",

  -- 2-character patterns
  ["ka"] = "か",
  ["ki"] = "き",
  ["ku"] = "く",
  ["ke"] = "け",
  ["ko"] = "こ",
  ["sa"] = "さ",
  ["si"] = "し",
  ["su"] = "す",
  ["se"] = "せ",
  ["so"] = "そ",
  ["ta"] = "た",
  ["ti"] = "ち",
  ["tu"] = "つ",
  ["te"] = "て",
  ["to"] = "と",
  ["na"] = "な",
  ["ni"] = "に",
  ["nu"] = "ぬ",
  ["ne"] = "ね",
  ["no"] = "の",
  ["ha"] = "は",
  ["hi"] = "ひ",
  ["hu"] = "ふ",
  ["fu"] = "ふ",
  ["he"] = "へ",
  ["ho"] = "ほ",
  ["ma"] = "ま",
  ["mi"] = "み",
  ["mu"] = "む",
  ["me"] = "め",
  ["mo"] = "も",
  ["ya"] = "や",
  ["yi"] = "い",
  ["yu"] = "ゆ",
  ["ye"] = "いぇ",
  ["yo"] = "よ",
  ["ra"] = "ら",
  ["ri"] = "り",
  ["ru"] = "る",
  ["re"] = "れ",
  ["ro"] = "ろ",
  ["wa"] = "わ",
  ["wi"] = "うぃ",
  ["we"] = "うぇ",
  ["wo"] = "を",
  ["nn"] = "ん",
  ["n'"] = "ん",
  ["xn"] = "ん",

  -- Voiced consonants (dakuon)
  ["ga"] = "が",
  ["gi"] = "ぎ",
  ["gu"] = "ぐ",
  ["ge"] = "げ",
  ["go"] = "ご",
  ["za"] = "ざ",
  ["zi"] = "じ",
  ["ji"] = "じ",
  ["zu"] = "ず",
  ["ze"] = "ぜ",
  ["zo"] = "ぞ",
  ["da"] = "だ",
  ["di"] = "ぢ",
  ["du"] = "づ",
  ["de"] = "で",
  ["do"] = "ど",
  ["ba"] = "ば",
  ["bi"] = "び",
  ["bu"] = "ぶ",
  ["be"] = "べ",
  ["bo"] = "ぼ",
  ["pa"] = "ぱ",
  ["pi"] = "ぴ",
  ["pu"] = "ぷ",
  ["pe"] = "ぺ",
  ["po"] = "ぽ",
  ["ja"] = "じゃ",
  ["ju"] = "じゅ",
  ["je"] = "じぇ",
  ["jo"] = "じょ",
  ["fa"] = "ふぁ",
  ["fi"] = "ふぃ",
  ["fe"] = "ふぇ",
  ["fo"] = "ふぉ",
  ["va"] = "ゔぁ",
  ["vi"] = "ゔぃ",
  ["vu"] = "ゔ",
  ["ve"] = "ゔぇ",
  ["vo"] = "ゔぉ",

  -- Small letters
  ["xa"] = "ぁ",
  ["xi"] = "ぃ",
  ["xu"] = "ぅ",
  ["xe"] = "ぇ",
  ["xo"] = "ぉ",
  ["la"] = "ぁ",
  ["li"] = "ぃ",
  ["lu"] = "ぅ",
  ["le"] = "ぇ",
  ["lo"] = "ぉ",

  -- 1-character patterns (vowels)
  ["a"] = "あ",
  ["i"] = "い",
  ["u"] = "う",
  ["e"] = "え",
  ["o"] = "お",

  -- Special
  ["-"] = "ー",
}

--- Consonants that can form sokuon (small tsu) when doubled
M.sokuon_consonants = {
  "k",
  "s",
  "t",
  "h",
  "m",
  "y",
  "r",
  "w",
  "g",
  "z",
  "d",
  "b",
  "p",
  "c",
  "f",
  "j",
}

--- Characters that follow 'n' without converting to 'ん'
M.n_continues = {
  "a",
  "i",
  "u",
  "e",
  "o",
  "y",
  "n",
}

--- Convert romaji input to hiragana
--- @param input string Romaji input
--- @return string hiragana Converted hiragana
--- @return string remaining Unconverted romaji remainder
function M.convert(input)
  local result = ""
  local pos = 1
  local len = #input

  while pos <= len do
    local matched = false

    -- Check for sokuon (doubled consonant)
    if pos + 1 <= len then
      local c1 = input:sub(pos, pos):lower()
      local c2 = input:sub(pos + 1, pos + 1):lower()
      if c1 == c2 and vim.tbl_contains(M.sokuon_consonants, c1) then
        result = result .. "っ"
        pos = pos + 1
        matched = true
      end
    end

    if not matched then
      -- Try longest match first (4 chars down to 1)
      for length = 4, 1, -1 do
        if pos + length - 1 <= len then
          local substr = input:sub(pos, pos + length - 1):lower()
          if M.table[substr] then
            result = result .. M.table[substr]
            pos = pos + length
            matched = true
            break
          end
        end
      end
    end

    -- Special handling for 'n'
    if not matched and input:sub(pos, pos):lower() == "n" then
      if pos == len then
        -- Trailing 'n' is kept as pending
        return result, input:sub(pos)
      else
        local next_char = input:sub(pos + 1, pos + 1):lower()
        if not vim.tbl_contains(M.n_continues, next_char) then
          -- 'n' followed by consonant becomes 'ん'
          result = result .. "ん"
          pos = pos + 1
          matched = true
        end
      end
    end

    if not matched then
      -- Cannot convert, return remaining as pending
      return result, input:sub(pos)
    end
  end

  return result, ""
end

--- Check if remaining input could potentially be converted
--- @param remaining string Unconverted romaji
--- @return boolean true if input is a valid prefix for conversion
function M.is_pending(remaining)
  if remaining == "" then
    return false
  end

  local lower = remaining:lower()

  -- Check if it's a prefix of any table key
  for key, _ in pairs(M.table) do
    if key:sub(1, #lower) == lower then
      return true
    end
  end

  -- Check for potential sokuon
  if #lower == 1 and vim.tbl_contains(M.sokuon_consonants, lower) then
    return true
  end

  return false
end

return M
