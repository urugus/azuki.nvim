//! Request handler and server state

use crate::config::load_dictionary;
use crate::converter::{AdjustDirection, Converter, Segment};
use crate::message::{Request, Response, SegmentInfo};

/// Server state
pub struct Server {
    converter: Converter,
}

impl Server {
    /// Create a new server with dictionary loaded from default paths
    pub fn new() -> Self {
        let dictionary = load_dictionary();
        let converter = Converter::new(dictionary);
        Self { converter }
    }

    /// Handle a request and return a response
    pub fn handle_request(&self, request: Request) -> Response {
        match request {
            Request::Init { seq, session_id } => {
                let session_id = session_id.unwrap_or_else(|| {
                    format!(
                        "session_{}",
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis()
                    )
                });
                Response::InitResult {
                    seq,
                    session_id,
                    version: env!("CARGO_PKG_VERSION").to_string(),
                    has_dictionary: self.converter.has_dictionary(),
                }
            }
            Request::Convert {
                seq,
                session_id,
                reading,
                cursor: _,
                options: _,
            } => {
                let result = self.converter.convert_with_segments(&reading);
                Response::ConvertResult {
                    seq,
                    session_id,
                    candidates: result.combined_candidates,
                    segments: result.segments.into_iter().map(SegmentInfo::from).collect(),
                }
            }
            Request::Commit {
                seq,
                session_id,
                reading: _,
                candidate: _,
            } => {
                // In the future, this will update learning data
                Response::CommitResult {
                    seq,
                    session_id,
                    success: true,
                }
            }
            Request::Shutdown { seq, .. } => Response::ShutdownResult { seq },
            Request::AdjustSegment {
                seq,
                session_id,
                reading,
                segments,
                segment_index,
                direction,
            } => {
                // Convert input segments to Segment structs
                let current_segments: Vec<Segment> = segments
                    .into_iter()
                    .map(|s| Segment {
                        reading: s.reading,
                        start: s.start,
                        length: s.length,
                        candidates: s.candidates,
                    })
                    .collect();

                // Parse direction
                let dir = match direction.as_str() {
                    "shrink" => AdjustDirection::Shrink,
                    "extend" => AdjustDirection::Extend,
                    _ => {
                        return Response::Error {
                            seq,
                            session_id: Some(session_id),
                            error: format!("Invalid direction: {}", direction),
                        };
                    }
                };

                // Adjust segments
                let new_segments =
                    self.converter
                        .adjust_segment(&reading, &current_segments, segment_index, dir);

                Response::AdjustSegmentResult {
                    seq,
                    session_id,
                    segments: new_segments.into_iter().map(SegmentInfo::from).collect(),
                }
            }
        }
    }
}

impl Default for Server {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_request() {
        let server = Server {
            converter: Converter::new(None),
        };
        let json = r#"{"type":"init","seq":1}"#;
        let request: Request = serde_json::from_str(json).unwrap();
        let response = server.handle_request(request);
        match response {
            Response::InitResult {
                seq,
                version,
                has_dictionary,
                ..
            } => {
                assert_eq!(seq, 1);
                assert!(!version.is_empty());
                assert!(!has_dictionary);
            }
            _ => panic!("Expected InitResult"),
        }
    }

    #[test]
    fn test_convert_request() {
        let server = Server {
            converter: Converter::new(None),
        };
        let json = r#"{"type":"convert","seq":42,"session_id":"abc","reading":"きょうは"}"#;
        let request: Request = serde_json::from_str(json).unwrap();
        let response = server.handle_request(request);
        match response {
            Response::ConvertResult {
                seq,
                session_id,
                candidates,
                ..
            } => {
                assert_eq!(seq, 42);
                assert_eq!(session_id, "abc");
                assert!(!candidates.is_empty());
                // Without dictionary, should return reading as-is
                assert_eq!(candidates[0], "きょうは");
            }
            _ => panic!("Expected ConvertResult"),
        }
    }

    #[test]
    fn test_shutdown_request() {
        let server = Server {
            converter: Converter::new(None),
        };
        let json = r#"{"type":"shutdown","seq":99}"#;
        let request: Request = serde_json::from_str(json).unwrap();
        let response = server.handle_request(request);
        match response {
            Response::ShutdownResult { seq } => {
                assert_eq!(seq, 99);
            }
            _ => panic!("Expected ShutdownResult"),
        }
    }
}
