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
        let mut in_okuri_nasi = false;

        for line in content.lines() {
            // Skip empty lines
            if line.is_empty() {
                continue;
            }

            // Check for section markers
            if line.starts_with(";; okuri-ari") {
                in_okuri_nasi = false;
                continue;
            }
            if line.starts_with(";; okuri-nasi") {
                in_okuri_nasi = true;
                continue;
            }

            // Skip comments
            if line.starts_with(';') {
                continue;
            }

            // Only process okuri-nasi entries for now
            if !in_okuri_nasi {
                continue;
            }

            // Parse entry: "reading /candidate1/candidate2/.../"
            if let Some((reading, candidates)) = parse_entry(line) {
                dict.okuri_nasi.insert(reading, candidates);
            }
        }

        eprintln!(
            "Loaded {} okuri-nasi entries from {}",
            dict.okuri_nasi.len(),
            path.display()
        );

        Ok(dict)
    }

    /// Look up candidates for a reading
    pub fn lookup(&self, reading: &str) -> Option<&Vec<String>> {
        self.okuri_nasi.get(reading)
    }

    /// Look up candidates with fallback to the reading itself
    ///
    /// Returns candidates from dictionary if found, otherwise returns the reading.
    /// Always includes the reading as the last candidate if not already present.
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

    /// Check if dictionary is empty
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.okuri_nasi.is_empty()
    }

    /// Get number of entries
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.okuri_nasi.len()
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
}
