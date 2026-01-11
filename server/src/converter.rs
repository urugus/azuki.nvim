//! Kana-kanji conversion logic

use crate::dictionary::Dictionary;

/// Kana-kanji converter
pub struct Converter {
    dictionary: Option<Dictionary>,
}

impl Converter {
    /// Create a new converter with optional dictionary
    pub fn new(dictionary: Option<Dictionary>) -> Self {
        Self { dictionary }
    }

    /// Convert hiragana reading to kanji candidates
    ///
    /// Uses longest-match algorithm to find candidates.
    /// Returns candidates for the entire reading, plus the reading itself as fallback.
    pub fn convert(&self, reading: &str) -> Vec<String> {
        if reading.is_empty() {
            return vec![];
        }

        let dict = match &self.dictionary {
            Some(d) => d,
            None => {
                // No dictionary, return reading as-is
                return vec![reading.to_string()];
            }
        };

        let reading_str = reading.to_string();

        // Try exact match first
        if let Some(candidates) = dict.lookup(reading) {
            let mut result = candidates.clone();
            // Add original reading as last fallback, if not already present
            if !result.iter().any(|c| c == &reading_str) {
                result.push(reading_str);
            }
            return result;
        }

        // Try segmented conversion using longest-match
        let segments = self.segment(reading);
        if segments.len() > 1 {
            // Combine first candidates from each segment
            let combined: String = segments
                .iter()
                .map(|candidates| candidates.first().unwrap().as_str())
                .collect();

            let mut result = vec![combined];
            // Add original reading as fallback
            result.push(reading.to_string());
            return result;
        }

        // No conversion found, return reading as-is
        vec![reading.to_string()]
    }

    /// Segment reading into convertible parts using longest-match
    fn segment(&self, reading: &str) -> Vec<Vec<String>> {
        let dict = match &self.dictionary {
            Some(d) => d,
            None => return vec![vec![reading.to_string()]],
        };

        let chars: Vec<char> = reading.chars().collect();
        let mut result = Vec::new();
        let mut pos = 0;

        while pos < chars.len() {
            let mut best_match: Option<(usize, &Vec<String>)> = None;

            // Try longest match first
            for end in (pos + 1..=chars.len()).rev() {
                let substr: String = chars[pos..end].iter().collect();
                if let Some(candidates) = dict.lookup(&substr) {
                    best_match = Some((end - pos, candidates));
                    break;
                }
            }

            match best_match {
                Some((len, candidates)) => {
                    // Clone only when adding to result
                    result.push(candidates.clone());
                    pos += len;
                }
                None => {
                    // No match, take single character
                    let ch: String = chars[pos..pos + 1].iter().collect();
                    result.push(vec![ch]);
                    pos += 1;
                }
            }
        }

        result
    }

    /// Check if dictionary is loaded
    pub fn has_dictionary(&self) -> bool {
        self.dictionary.is_some()
    }
}

impl Default for Converter {
    fn default() -> Self {
        Self::new(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_dict_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/test-dict.utf8")
    }

    fn load_test_dictionary() -> Dictionary {
        Dictionary::load(test_dict_path()).unwrap()
    }

    #[test]
    fn test_convert_no_dictionary() {
        let converter = Converter::new(None);
        let result = converter.convert("きょう");
        assert_eq!(result, vec!["きょう"]);
    }

    #[test]
    fn test_convert_empty() {
        let converter = Converter::new(None);
        let result = converter.convert("");
        assert!(result.is_empty());
    }

    #[test]
    fn test_convert_with_dictionary() {
        let dict = load_test_dictionary();
        let converter = Converter::new(Some(dict));

        // Exact match
        let result = converter.convert("きょう");
        assert!(result.contains(&"今日".to_string()));
        assert!(result.contains(&"きょう".to_string())); // fallback

        // Another exact match
        let result = converter.convert("あずき");
        assert!(result.contains(&"小豆".to_string()));
    }

    #[test]
    fn test_convert_segmented() {
        let dict = load_test_dictionary();
        let converter = Converter::new(Some(dict));

        // "きょうは" should segment to "きょう" + "は"
        // "きょう" -> "今日", "は" -> no match, stays as-is
        let result = converter.convert("きょうは");
        // First result should be combined: "今日" + "は" = "今日は"
        assert!(result.iter().any(|c| c.contains("今日")));
    }

    #[test]
    fn test_convert_no_match() {
        let dict = load_test_dictionary();
        let converter = Converter::new(Some(dict));

        // No match in dictionary
        let result = converter.convert("あいうえお");
        assert!(result.contains(&"あいうえお".to_string()));
    }
}
