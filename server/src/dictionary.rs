//! SKK dictionary loader and lookup

use encoding_rs::{EUC_JP, UTF_8};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// SKK dictionary
#[derive(Debug, Default)]
pub struct Dictionary {
    /// Okuri-nasi entries (without okurigana)
    /// Key: reading (hiragana), Value: list of candidates
    okuri_nasi: HashMap<String, Vec<String>>,
    /// Okuri-ari entries (with okurigana)
    /// Key: reading + okuri symbol (e.g., "かk"), Value: list of kanji stems
    okuri_ari: HashMap<String, Vec<String>>,
}

impl Dictionary {
    /// Create a new empty dictionary
    pub fn new() -> Self {
        Self::default()
    }

    /// Load dictionary from file
    ///
    /// Supports both EUC-JP and UTF-8 encoded files.
    /// The encoding is auto-detected: UTF-8 is tried first, then EUC-JP.
    ///
    /// SKK dictionary format:
    /// - Lines starting with `;` are comments
    /// - Entry format: `reading /candidate1/candidate2/.../`
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, DictionaryError> {
        let path = path.as_ref();
        let bytes = fs::read(path).map_err(|e| DictionaryError::Io(e.to_string()))?;

        // Try UTF-8 first, then EUC-JP
        let (content, encoding_name) = decode_content(&bytes);

        eprintln!(
            "Loading dictionary from {} (detected encoding: {})",
            path.display(),
            encoding_name
        );

        let mut dict = Self::new();
        // SKK dictionaries start with okuri-ari section
        let mut in_okuri_ari = true;
        let mut in_okuri_nasi = false;

        for line in content.lines() {
            // Skip empty lines
            if line.is_empty() {
                continue;
            }

            // Check for section markers
            if line.starts_with(";; okuri-ari") {
                in_okuri_ari = true;
                in_okuri_nasi = false;
                continue;
            }
            if line.starts_with(";; okuri-nasi") {
                in_okuri_ari = false;
                in_okuri_nasi = true;
                continue;
            }

            // Skip comments
            if line.starts_with(';') {
                continue;
            }

            // Parse entry: "reading /candidate1/candidate2/.../"
            if let Some((reading, candidates)) = parse_entry(line) {
                if in_okuri_ari {
                    dict.okuri_ari.insert(reading, candidates);
                } else if in_okuri_nasi {
                    dict.okuri_nasi.insert(reading, candidates);
                }
            }
        }

        eprintln!(
            "Loaded {} okuri-nasi, {} okuri-ari entries from {}",
            dict.okuri_nasi.len(),
            dict.okuri_ari.len(),
            path.display()
        );

        Ok(dict)
    }

    /// Look up candidates for a reading (okuri-nasi only)
    #[allow(dead_code)]
    pub fn lookup(&self, reading: &str) -> Option<&Vec<String>> {
        self.okuri_nasi.get(reading)
    }

    /// Look up candidates with fallback to the reading itself (okuri-nasi only)
    ///
    /// Returns candidates from dictionary if found, otherwise returns the reading.
    /// Always includes the reading as the last candidate if not already present.
    #[allow(dead_code)]
    pub fn lookup_with_fallback(&self, reading: &str) -> Vec<String> {
        match self.okuri_nasi.get(reading) {
            Some(candidates) => {
                let mut result = candidates.clone();
                if !result.contains(&reading.to_string()) {
                    result.push(reading.to_string());
                }
                result
            }
            None => vec![reading.to_string()],
        }
    }

    /// Look up okuri-ari candidates
    ///
    /// Arguments:
    /// - stem: The reading stem without okuri (e.g., "か" for "書く")
    /// - okuri_char: The first character of okurigana (e.g., 'く')
    ///
    /// Returns kanji stems if found (e.g., ["書", "欠"] for stem="か", okuri_char='く')
    pub fn lookup_okuri_ari(&self, stem: &str, okuri_char: char) -> Option<&Vec<String>> {
        let okuri_symbol = hiragana_to_okuri_symbol(okuri_char)?;
        let key = format!("{}{}", stem, okuri_symbol);
        self.okuri_ari.get(&key)
    }

    /// Look up candidates including both okuri-nasi and okuri-ari entries
    ///
    /// For a reading like "かく":
    /// 1. Looks up okuri-nasi "かく" -> returns direct candidates
    /// 2. Tries okuri-ari with stem="か", okuri='く' -> returns "書く", "欠く", etc.
    ///
    /// Returns a combined list with okuri-nasi candidates first, then okuri-ari
    pub fn lookup_combined(&self, reading: &str) -> Vec<String> {
        let mut result = Vec::new();

        // 1. okuri-nasi lookup
        if let Some(candidates) = self.okuri_nasi.get(reading) {
            result.extend(candidates.clone());
        }

        // 2. okuri-ari lookup (try last 1 character as okuri)
        let chars: Vec<char> = reading.chars().collect();
        if chars.len() >= 2 {
            let stem: String = chars[..chars.len() - 1].iter().collect();
            let okuri_char = chars[chars.len() - 1];

            if let Some(kanji_stems) = self.lookup_okuri_ari(&stem, okuri_char) {
                // Build full forms: kanji_stem + okuri_char
                for kanji_stem in kanji_stems {
                    let full_form = format!("{}{}", kanji_stem, okuri_char);
                    if !result.contains(&full_form) {
                        result.push(full_form);
                    }
                }
            }
        }

        // Add original reading as fallback if not empty and not already present
        if !reading.is_empty() && !result.contains(&reading.to_string()) {
            result.push(reading.to_string());
        }

        result
    }

    /// Check if dictionary has any candidates for the given reading
    ///
    /// Returns true if either okuri-nasi has the reading, or okuri-ari can match
    /// (i.e., last character as okuri gives a match)
    pub fn has_candidates(&self, reading: &str) -> bool {
        // Check okuri-nasi
        if self.okuri_nasi.contains_key(reading) {
            return true;
        }

        // Check okuri-ari (last 1 character as okuri)
        let chars: Vec<char> = reading.chars().collect();
        if chars.len() >= 2 {
            let stem: String = chars[..chars.len() - 1].iter().collect();
            let okuri_char = chars[chars.len() - 1];
            if let Some(okuri_symbol) = hiragana_to_okuri_symbol(okuri_char) {
                let key = format!("{}{}", stem, okuri_symbol);
                if self.okuri_ari.contains_key(&key) {
                    return true;
                }
            }
        }

        false
    }

    /// Check if dictionary is empty
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.okuri_nasi.is_empty() && self.okuri_ari.is_empty()
    }

    /// Get number of entries (okuri-nasi + okuri-ari)
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.okuri_nasi.len() + self.okuri_ari.len()
    }

    /// Get number of okuri-ari entries
    #[allow(dead_code)]
    pub fn okuri_ari_len(&self) -> usize {
        self.okuri_ari.len()
    }
}

/// Decode file content, trying UTF-8 first, then EUC-JP
fn decode_content(bytes: &[u8]) -> (String, &'static str) {
    // Try UTF-8 first
    let (decoded, encoding, had_errors) = UTF_8.decode(bytes);
    if !had_errors {
        return (decoded.into_owned(), encoding.name());
    }

    // Fall back to EUC-JP
    let (decoded, encoding, _) = EUC_JP.decode(bytes);
    (decoded.into_owned(), encoding.name())
}

/// Parse a single dictionary entry
/// Format: "reading /candidate1/candidate2/.../"
fn parse_entry(line: &str) -> Option<(String, Vec<String>)> {
    // Find the first space that separates reading from candidates
    let space_pos = line.find(' ')?;
    let reading = line[..space_pos].to_string();
    let rest = &line[space_pos + 1..];

    // Extract candidates between slashes
    let mut candidates = Vec::new();
    for part in rest.split('/') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        // Skip entries with annotations (marked with ;)
        // e.g., "候補;annotation" -> "候補"
        let candidate = part.split(';').next().unwrap_or(part).to_string();
        if !candidate.is_empty() {
            candidates.push(candidate);
        }
    }

    if candidates.is_empty() {
        return None;
    }

    Some((reading, candidates))
}

/// Convert hiragana to okuri symbol for dictionary lookup
///
/// SKK dictionaries use consonant symbols for okuri-ari entries.
/// For example, "かk" means "か" + any kana starting with "k" (か, き, く, け, こ).
pub fn hiragana_to_okuri_symbol(c: char) -> Option<char> {
    match c {
        // Vowels: use the hiragana itself
        'あ' | 'い' | 'う' | 'え' | 'お' => Some(c),
        // K-row
        'か' | 'き' | 'く' | 'け' | 'こ' => Some('k'),
        // S-row
        'さ' | 'し' | 'す' | 'せ' | 'そ' => Some('s'),
        // T-row
        'た' | 'ち' | 'つ' | 'て' | 'と' => Some('t'),
        // N-row
        'な' | 'に' | 'ぬ' | 'ね' | 'の' => Some('n'),
        // H-row
        'は' | 'ひ' | 'ふ' | 'へ' | 'ほ' => Some('h'),
        // M-row
        'ま' | 'み' | 'む' | 'め' | 'も' => Some('m'),
        // Y-row
        'や' | 'ゆ' | 'よ' => Some('y'),
        // R-row
        'ら' | 'り' | 'る' | 'れ' | 'ろ' => Some('r'),
        // W-row (including ん)
        'わ' | 'を' | 'ん' => Some('w'),
        // G-row (voiced K)
        'が' | 'ぎ' | 'ぐ' | 'げ' | 'ご' => Some('g'),
        // Z-row (voiced S)
        'ざ' | 'じ' | 'ず' | 'ぜ' | 'ぞ' => Some('z'),
        // D-row (voiced T)
        'だ' | 'ぢ' | 'づ' | 'で' | 'ど' => Some('d'),
        // B-row (voiced H)
        'ば' | 'び' | 'ぶ' | 'べ' | 'ぼ' => Some('b'),
        // P-row (semi-voiced H)
        'ぱ' | 'ぴ' | 'ぷ' | 'ぺ' | 'ぽ' => Some('p'),
        // Small kana - map to their base row
        'ぁ' | 'ぃ' | 'ぅ' | 'ぇ' | 'ぉ' => Some(c),
        'ゃ' | 'ゅ' | 'ょ' => Some('y'),
        'っ' => Some('t'),
        _ => None,
    }
}

/// Dictionary error
#[derive(Debug)]
#[allow(dead_code)]
pub enum DictionaryError {
    Io(String),
    Parse(String),
}

impl std::fmt::Display for DictionaryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DictionaryError::Io(e) => write!(f, "IO error: {}", e),
            DictionaryError::Parse(e) => write!(f, "Parse error: {}", e),
        }
    }
}

impl std::error::Error for DictionaryError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_dict_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/test-dict.utf8")
    }

    #[test]
    fn test_parse_entry() {
        let (reading, candidates) = parse_entry("きょう /今日/京/教/").unwrap();
        assert_eq!(reading, "きょう");
        assert_eq!(candidates, vec!["今日", "京", "教"]);
    }

    #[test]
    fn test_parse_entry_with_annotation() {
        let (reading, candidates) = parse_entry("かんじ /漢字;kanji/感じ/").unwrap();
        assert_eq!(reading, "かんじ");
        assert_eq!(candidates, vec!["漢字", "感じ"]);
    }

    #[test]
    fn test_parse_entry_empty() {
        assert!(parse_entry("invalid").is_none());
        assert!(parse_entry("reading //").is_none());
    }

    #[test]
    fn test_load_dictionary_utf8() {
        let dict = Dictionary::load(test_dict_path()).unwrap();
        assert!(!dict.is_empty());

        // Test lookup
        let candidates = dict.lookup("きょう").unwrap();
        assert_eq!(candidates, &vec!["今日", "京", "教"]);

        let candidates = dict.lookup("あずき").unwrap();
        assert_eq!(candidates, &vec!["小豆"]);

        // Non-existent entry
        assert!(dict.lookup("そんざいしない").is_none());
    }

    #[test]
    fn test_decode_content_utf8() {
        let utf8_bytes = "きょう /今日/".as_bytes();
        let (decoded, encoding) = decode_content(utf8_bytes);
        assert_eq!(decoded, "きょう /今日/");
        assert_eq!(encoding, "UTF-8");
    }

    #[test]
    fn test_decode_content_eucjp() {
        // EUC-JP encoded "きょう /今日/"
        // きょう = 0xA4 0xAD 0xA4 0xE7 0xA4 0xA6
        // 今日 = 0xBA 0xA3 0xC6 0xFC
        let eucjp_bytes: Vec<u8> = vec![
            0xA4, 0xAD, 0xA4, 0xE7, 0xA4, 0xA6, // きょう
            0x20, 0x2F, // " /"
            0xBA, 0xA3, 0xC6, 0xFC, // 今日
            0x2F, // "/"
        ];
        let (decoded, encoding) = decode_content(&eucjp_bytes);
        assert_eq!(decoded, "きょう /今日/");
        assert_eq!(encoding, "EUC-JP");
    }

    #[test]
    fn test_lookup_with_fallback_found() {
        let dict = Dictionary::load(test_dict_path()).unwrap();
        // Entry exists: should return candidates with reading as fallback
        let result = dict.lookup_with_fallback("きょう");
        assert!(result.contains(&"今日".to_string()));
        assert!(result.contains(&"きょう".to_string())); // fallback included
    }

    #[test]
    fn test_lookup_with_fallback_not_found() {
        let dict = Dictionary::load(test_dict_path()).unwrap();
        // Entry does not exist: should return reading only
        let result = dict.lookup_with_fallback("そんざいしない");
        assert_eq!(result, vec!["そんざいしない"]);
    }

    #[test]
    fn test_lookup_with_fallback_empty_string() {
        let dict = Dictionary::load(test_dict_path()).unwrap();
        // Empty string: should return empty string only
        let result = dict.lookup_with_fallback("");
        assert_eq!(result, vec![""]);
    }

    #[test]
    fn test_hiragana_to_okuri_symbol() {
        // K-row
        assert_eq!(hiragana_to_okuri_symbol('か'), Some('k'));
        assert_eq!(hiragana_to_okuri_symbol('き'), Some('k'));
        assert_eq!(hiragana_to_okuri_symbol('く'), Some('k'));
        // S-row
        assert_eq!(hiragana_to_okuri_symbol('す'), Some('s'));
        // G-row (voiced)
        assert_eq!(hiragana_to_okuri_symbol('が'), Some('g'));
        // Vowels
        assert_eq!(hiragana_to_okuri_symbol('あ'), Some('あ'));
        assert_eq!(hiragana_to_okuri_symbol('い'), Some('い'));
        // M-row
        assert_eq!(hiragana_to_okuri_symbol('む'), Some('m'));
        // Unknown
        assert_eq!(hiragana_to_okuri_symbol('ー'), None);
    }

    #[test]
    fn test_load_okuri_ari() {
        let dict = Dictionary::load(test_dict_path()).unwrap();
        // Should have okuri-ari entries
        assert!(dict.okuri_ari_len() > 0);
    }

    #[test]
    fn test_lookup_okuri_ari() {
        let dict = Dictionary::load(test_dict_path()).unwrap();

        // "か" + "く" -> "かk" -> ["書", "欠"]
        let candidates = dict.lookup_okuri_ari("か", 'く').unwrap();
        assert!(candidates.contains(&"書".to_string()));
        assert!(candidates.contains(&"欠".to_string()));

        // "うご" + "く" -> "うごk" -> ["動"]
        let candidates = dict.lookup_okuri_ari("うご", 'く').unwrap();
        assert!(candidates.contains(&"動".to_string()));

        // "よ" + "む" -> "よm" -> ["読"]
        let candidates = dict.lookup_okuri_ari("よ", 'む').unwrap();
        assert!(candidates.contains(&"読".to_string()));

        // Non-existent
        assert!(dict.lookup_okuri_ari("そんざい", 'く').is_none());
    }

    #[test]
    fn test_lookup_combined() {
        let dict = Dictionary::load(test_dict_path()).unwrap();

        // "かく" -> okuri-nasi has no match, okuri-ari has "書く", "欠く"
        let result = dict.lookup_combined("かく");
        assert!(result.contains(&"書く".to_string()));
        assert!(result.contains(&"欠く".to_string()));
        assert!(result.contains(&"かく".to_string())); // fallback

        // "よむ" -> okuri-ari has "読む"
        let result = dict.lookup_combined("よむ");
        assert!(result.contains(&"読む".to_string()));

        // "きょう" -> okuri-nasi has match
        let result = dict.lookup_combined("きょう");
        assert!(result.contains(&"今日".to_string()));
    }

    #[test]
    fn test_lookup_combined_single_char() {
        let dict = Dictionary::load(test_dict_path()).unwrap();
        // Single character should just return itself
        let result = dict.lookup_combined("あ");
        assert_eq!(result, vec!["あ"]);
    }
}
