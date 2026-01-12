//! Request handler and server state

use crate::config::load_dictionary;
use crate::converter::{AdjustDirection, Converter, Segment};
use crate::message::{Request, Response, SegmentInfo};
#[cfg(feature = "zenzai")]
use crate::zenzai::ZenzaiBackend;
use crate::zenzai::ZenzaiConfig;

/// Server state
pub struct Server {
    converter: Converter,
    #[cfg(feature = "zenzai")]
    zenzai: Option<ZenzaiBackend>,
    #[cfg(not(feature = "zenzai"))]
    #[allow(dead_code)]
    zenzai_config: Option<ZenzaiConfig>,
}

impl Server {
    /// Create a new server with dictionary loaded from default paths
    pub fn new() -> Self {
        let dictionary = load_dictionary();
        let converter = Converter::new(dictionary);
        Self {
            converter,
            #[cfg(feature = "zenzai")]
            zenzai: None,
            #[cfg(not(feature = "zenzai"))]
            zenzai_config: None,
        }
    }

    /// Initialize Zenzai backend if configured
    #[cfg(feature = "zenzai")]
    fn init_zenzai(&mut self, config: ZenzaiConfig) -> bool {
        if !config.enabled {
            eprintln!("[zenzai] Disabled by configuration");
            return false;
        }

        if !config.is_usable() {
            eprintln!("[zenzai] Model not found, falling back to dictionary-based conversion");
            return false;
        }

        let mut backend = ZenzaiBackend::new(config);
        match backend.initialize() {
            Ok(()) => {
                self.zenzai = Some(backend);
                eprintln!("[zenzai] Initialized successfully");
                true
            }
            Err(e) => {
                eprintln!("[zenzai] Initialization failed: {}", e);
                false
            }
        }
    }

    /// Check if Zenzai is enabled and ready
    #[cfg(feature = "zenzai")]
    fn is_zenzai_enabled(&self) -> bool {
        self.zenzai.as_ref().is_some_and(|z| z.is_ready())
    }

    #[cfg(not(feature = "zenzai"))]
    #[allow(dead_code)]
    fn is_zenzai_enabled(&self) -> bool {
        false
    }

    /// Handle a request and return a response
    pub fn handle_request(&mut self, request: Request) -> Response {
        match request {
            Request::Init {
                seq,
                session_id,
                zenzai,
            } => {
                let session_id = session_id.unwrap_or_else(|| {
                    format!(
                        "session_{}",
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis()
                    )
                });

                // Initialize Zenzai if requested
                // Can't use map() here due to #[cfg] attributes inside
                #[allow(clippy::manual_map)]
                let zenzai_enabled = if let Some(config) = zenzai {
                    #[cfg(feature = "zenzai")]
                    {
                        Some(self.init_zenzai(config))
                    }
                    #[cfg(not(feature = "zenzai"))]
                    {
                        self.zenzai_config = Some(config);
                        eprintln!("[zenzai] Feature not enabled at compile time");
                        Some(false)
                    }
                } else {
                    None
                };

                Response::InitResult {
                    seq,
                    session_id,
                    version: env!("CARGO_PKG_VERSION").to_string(),
                    has_dictionary: self.converter.has_dictionary(),
                    zenzai_enabled,
                }
            }
            Request::Convert {
                seq,
                session_id,
                reading,
                cursor: _,
                options: _,
            } => {
                // Try Zenzai first if enabled
                #[cfg(feature = "zenzai")]
                let zenzai_candidates = if self.is_zenzai_enabled() {
                    if let Some(ref mut zenzai) = self.zenzai {
                        match zenzai.convert(&reading, None) {
                            Ok(candidates) => {
                                eprintln!("[handler] Zenzai conversion successful");
                                Some(candidates)
                            }
                            Err(e) => {
                                eprintln!("[handler] Zenzai conversion failed: {}, falling back to dictionary", e);
                                None
                            }
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };

                #[cfg(not(feature = "zenzai"))]
                let zenzai_candidates: Option<Vec<String>> = None;

                // Get dictionary-based result for segments
                let dict_result = self.converter.convert_with_segments(&reading);

                // Merge candidates: Zenzai first, then dictionary
                let candidates = if let Some(mut zenzai_cands) = zenzai_candidates {
                    // Add dictionary candidates that aren't already in Zenzai results
                    for cand in dict_result.combined_candidates.iter() {
                        if !zenzai_cands.contains(cand) {
                            zenzai_cands.push(cand.clone());
                        }
                    }
                    zenzai_cands
                } else {
                    dict_result.combined_candidates
                };

                Response::ConvertResult {
                    seq,
                    session_id,
                    candidates,
                    segments: dict_result
                        .segments
                        .into_iter()
                        .map(SegmentInfo::from)
                        .collect(),
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

    fn create_test_server() -> Server {
        Server {
            converter: Converter::new(None),
            #[cfg(feature = "zenzai")]
            zenzai: None,
            #[cfg(not(feature = "zenzai"))]
            zenzai_config: None,
        }
    }

    #[test]
    fn test_init_request() {
        let mut server = create_test_server();
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
        let mut server = create_test_server();
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
        let mut server = create_test_server();
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

    #[test]
    fn test_init_with_zenzai_config() {
        let mut server = create_test_server();
        let json = r#"{"type":"init","seq":1,"zenzai":{"enabled":true}}"#;
        let request: Request = serde_json::from_str(json).unwrap();
        let response = server.handle_request(request);
        match response {
            Response::InitResult {
                seq,
                zenzai_enabled,
                ..
            } => {
                assert_eq!(seq, 1);
                // Without model file, zenzai should not be enabled
                assert_eq!(zenzai_enabled, Some(false));
            }
            _ => panic!("Expected InitResult"),
        }
    }
}
