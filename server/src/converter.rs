//! Kana-kanji conversion logic

use crate::dictionary::Dictionary;
use serde::Serialize;

/// Segment information for UI display
#[derive(Debug, Clone, Serialize)]
pub struct Segment {
    /// Reading (hiragana) for this segment
    pub reading: String,
    /// Start position in the original reading (character index)
    pub start: usize,
    /// Length of this segment (character count)
    pub length: usize,
    /// Conversion candidates for this segment
    pub candidates: Vec<String>,
}

/// Conversion result with segment information
#[derive(Debug, Clone, Serialize)]
pub struct ConversionResult {
    /// Combined candidates (first candidate from each segment joined)
    pub combined_candidates: Vec<String>,
    /// Individual segment information
    pub segments: Vec<Segment>,
}

/// Direction for segment boundary adjustment
#[derive(Debug, Clone, Copy)]
pub enum AdjustDirection {
    /// Make segment shorter (move boundary left)
    Shrink,
    /// Make segment longer (move boundary right)
    Extend,
}

/// Kana-kanji converter
pub struct Converter {
    dictionary: Option<Dictionary>,
}

impl Converter {
    /// Create a new converter with optional dictionary
    pub fn new(dictionary: Option<Dictionary>) -> Self {
        Self { dictionary }
    }

    /// Segment reading into convertible parts with position information
    pub fn segment_with_info(&self, reading: &str) -> Vec<Segment> {
        let dict = match &self.dictionary {
            Some(d) => d,
            None => {
                // No dictionary, return entire reading as one segment
                return vec![Segment {
                    reading: reading.to_string(),
                    start: 0,
                    length: reading.chars().count(),
                    candidates: vec![reading.to_string()],
                }];
            }
        };

        let chars: Vec<char> = reading.chars().collect();
        let mut segments = Vec::new();
        let mut pos = 0;

        while pos < chars.len() {
            let mut best_match: Option<(usize, String)> = None;

            // Try longest match first
            for end in (pos + 1..=chars.len()).rev() {
                let substr: String = chars[pos..end].iter().collect();
                if dict.lookup(&substr).is_some() {
                    best_match = Some((end - pos, substr));
                    break;
                }
            }

            match best_match {
                Some((len, seg_reading)) => {
                    let candidates = dict.lookup_with_fallback(&seg_reading);
                    segments.push(Segment {
                        reading: seg_reading,
                        start: pos,
                        length: len,
                        candidates,
                    });
                    pos += len;
                }
                None => {
                    // No match, take single character
                    let ch: String = chars[pos..pos + 1].iter().collect();
                    segments.push(Segment {
                        reading: ch.clone(),
                        start: pos,
                        length: 1,
                        candidates: vec![ch],
                    });
                    pos += 1;
                }
            }
        }

        segments
    }

    /// Convert with segment information
    pub fn convert_with_segments(&self, reading: &str) -> ConversionResult {
        if reading.is_empty() {
            return ConversionResult {
                combined_candidates: vec![],
                segments: vec![],
            };
        }

        let segments = self.segment_with_info(reading);

        // Combine first candidates from each segment
        let combined: String = segments
            .iter()
            .map(|s| s.candidates.first().unwrap_or(&s.reading).as_str())
            .collect();

        let mut combined_candidates = vec![combined];
        // Add original reading as fallback
        if combined_candidates[0] != reading {
            combined_candidates.push(reading.to_string());
        }

        ConversionResult {
            combined_candidates,
            segments,
        }
    }

    /// Check if segment adjustment is possible
    fn can_adjust(&self, segments: &[Segment], index: usize, direction: AdjustDirection) -> bool {
        // Cannot adjust last segment (no next segment to exchange with)
        if index >= segments.len() - 1 {
            return false;
        }

        match direction {
            AdjustDirection::Shrink => segments[index].length > 1,
            AdjustDirection::Extend => segments[index + 1].length > 1,
        }
    }

    /// Adjust segment boundary
    ///
    /// Returns new segments after adjusting the boundary of the specified segment.
    /// - Shrink: Move one character from this segment to the next
    /// - Extend: Take one character from the next segment
    pub fn adjust_segment(
        &self,
        reading: &str,
        current_segments: &[Segment],
        segment_index: usize,
        direction: AdjustDirection,
    ) -> Vec<Segment> {
        let chars: Vec<char> = reading.chars().collect();

        // Validate segment index
        if segment_index >= current_segments.len() {
            return current_segments.to_vec();
        }

        // Check if adjustment is possible
        if !self.can_adjust(current_segments, segment_index, direction) {
            return current_segments.to_vec();
        }

        // Calculate new boundaries based on direction
        let new_boundaries = match direction {
            AdjustDirection::Shrink => {
                self.calculate_shrink_boundaries(current_segments, segment_index)
            }
            AdjustDirection::Extend => {
                self.calculate_extend_boundaries(current_segments, segment_index)
            }
        };

        // Rebuild segments with new boundaries
        self.rebuild_segments_from_boundaries(&chars, &new_boundaries)
    }

    /// Calculate boundaries after shrinking a segment
    fn calculate_shrink_boundaries(&self, segments: &[Segment], index: usize) -> Vec<usize> {
        let mut boundaries = Vec::new();
        let mut pos = 0;

        for (i, seg) in segments.iter().enumerate() {
            if i == index {
                // This segment loses one character
                pos += seg.length - 1;
            } else if i == index + 1 {
                // Next segment gains one character (boundary moved left)
                // The boundary is already adjusted by the previous segment
                pos += seg.length + 1;
            } else {
                pos += seg.length;
            }
            boundaries.push(pos);
        }

        boundaries
    }

    /// Calculate boundaries after extending a segment
    fn calculate_extend_boundaries(&self, segments: &[Segment], index: usize) -> Vec<usize> {
        let mut boundaries = Vec::new();
        let mut pos = 0;

        for (i, seg) in segments.iter().enumerate() {
            if i == index {
                // This segment gains one character
                pos += seg.length + 1;
            } else if i == index + 1 {
                // Next segment loses one character
                pos += seg.length - 1;
            } else {
                pos += seg.length;
            }
            boundaries.push(pos);
        }

        boundaries
    }

    /// Rebuild segments from character boundaries
    fn rebuild_segments_from_boundaries(
        &self,
        chars: &[char],
        boundaries: &[usize],
    ) -> Vec<Segment> {
        let mut segments = Vec::new();
        let mut start = 0;

        for &end in boundaries.iter() {
            if start >= chars.len() || end <= start {
                continue;
            }

            let seg_reading: String = chars[start..end].iter().collect();
            let candidates = match &self.dictionary {
                Some(dict) => dict.lookup_with_fallback(&seg_reading),
                None => vec![seg_reading.clone()],
            };

            segments.push(Segment {
                reading: seg_reading,
                start,
                length: end - start,
                candidates,
            });

            start = end;
        }

        segments
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
        let result = converter.convert_with_segments("きょう");
        assert_eq!(result.combined_candidates, vec!["きょう"]);
    }

    #[test]
    fn test_convert_empty() {
        let converter = Converter::new(None);
        let result = converter.convert_with_segments("");
        assert!(result.combined_candidates.is_empty());
    }

    #[test]
    fn test_convert_with_dictionary() {
        let dict = load_test_dictionary();
        let converter = Converter::new(Some(dict));

        // Exact match
        let result = converter.convert_with_segments("きょう");
        assert!(result.combined_candidates.iter().any(|c| c == "今日"));

        // Another exact match
        let result = converter.convert_with_segments("あずき");
        assert!(result.combined_candidates.iter().any(|c| c == "小豆"));
    }

    #[test]
    fn test_convert_segmented() {
        let dict = load_test_dictionary();
        let converter = Converter::new(Some(dict));

        // "きょうは" should segment to "きょう" + "は"
        // "きょう" -> "今日", "は" -> no match, stays as-is
        let result = converter.convert_with_segments("きょうは");
        // First result should be combined: "今日" + "は" = "今日は"
        assert!(result
            .combined_candidates
            .iter()
            .any(|c| c.contains("今日")));
    }

    #[test]
    fn test_convert_no_match() {
        let dict = load_test_dictionary();
        let converter = Converter::new(Some(dict));

        // No match in dictionary
        let result = converter.convert_with_segments("あいうえお");
        assert!(result.combined_candidates.iter().any(|c| c == "あいうえお"));
    }
}
