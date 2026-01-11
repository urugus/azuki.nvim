mod converter;
mod dictionary;

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use converter::{AdjustDirection, Converter, Segment};
use dictionary::Dictionary;
use serde::{Deserialize, Serialize};
use std::io::{self, BufReader, Read, Write};
use std::path::PathBuf;

/// Maximum message size (4MB)
const MAX_MESSAGE_SIZE: u32 = 4 * 1024 * 1024;

/// Default dictionary paths to search
fn default_dictionary_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // XDG data home
    if let Ok(data_home) = std::env::var("XDG_DATA_HOME") {
        paths.push(PathBuf::from(data_home).join("azuki/dict/SKK-JISYO.L"));
    }

    // Home directory fallback
    if let Ok(home) = std::env::var("HOME") {
        paths.push(PathBuf::from(&home).join(".local/share/azuki/dict/SKK-JISYO.L"));
        paths.push(PathBuf::from(&home).join(".azuki/dict/SKK-JISYO.L"));
    }

    // System paths
    paths.push(PathBuf::from("/usr/share/skk/SKK-JISYO.L"));
    paths.push(PathBuf::from("/usr/local/share/skk/SKK-JISYO.L"));

    paths
}

/// Find and load dictionary from default paths
fn load_dictionary() -> Option<Dictionary> {
    // Check environment variable first
    if let Ok(dict_path) = std::env::var("AZUKI_DICTIONARY") {
        match Dictionary::load(&dict_path) {
            Ok(dict) => {
                eprintln!("Loaded dictionary from AZUKI_DICTIONARY: {}", dict_path);
                return Some(dict);
            }
            Err(e) => {
                eprintln!("Failed to load dictionary from {}: {}", dict_path, e);
            }
        }
    }

    // Search default paths
    for path in default_dictionary_paths() {
        if path.exists() {
            match Dictionary::load(&path) {
                Ok(dict) => {
                    eprintln!("Loaded dictionary from: {}", path.display());
                    return Some(dict);
                }
                Err(e) => {
                    eprintln!("Failed to load dictionary from {}: {}", path.display(), e);
                }
            }
        }
    }

    eprintln!("No dictionary found. Running without dictionary (hiragana pass-through mode).");
    None
}

// Request types
// Fields marked with allow(dead_code) will be used in future phases
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[allow(dead_code)]
enum Request {
    Init {
        seq: u64,
        #[serde(default)]
        session_id: Option<String>,
    },
    Convert {
        seq: u64,
        session_id: String,
        reading: String,
        #[serde(default)]
        cursor: Option<usize>,
        #[serde(default)]
        options: Option<ConvertOptions>,
    },
    Commit {
        seq: u64,
        session_id: String,
        reading: String,
        candidate: String,
    },
    Shutdown {
        seq: u64,
        #[serde(default)]
        session_id: Option<String>,
    },
    AdjustSegment {
        seq: u64,
        session_id: String,
        reading: String,
        segments: Vec<SegmentInput>,
        segment_index: usize,
        direction: String,
    },
}

/// Input segment for adjust_segment request
#[derive(Debug, Deserialize)]
struct SegmentInput {
    reading: String,
    start: usize,
    length: usize,
    candidates: Vec<String>,
}

// Options will be used in future phases for live conversion settings
#[derive(Debug, Deserialize, Default)]
#[allow(dead_code)]
struct ConvertOptions {
    #[serde(default)]
    live: bool,
}

/// Segment info for response
#[derive(Debug, Serialize)]
struct SegmentInfo {
    reading: String,
    start: usize,
    length: usize,
    candidates: Vec<String>,
}

impl From<Segment> for SegmentInfo {
    fn from(seg: Segment) -> Self {
        Self {
            reading: seg.reading,
            start: seg.start,
            length: seg.length,
            candidates: seg.candidates,
        }
    }
}

// Response types
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Response {
    InitResult {
        seq: u64,
        session_id: String,
        version: String,
        has_dictionary: bool,
    },
    ConvertResult {
        seq: u64,
        session_id: String,
        candidates: Vec<String>,
        segments: Vec<SegmentInfo>,
    },
    AdjustSegmentResult {
        seq: u64,
        session_id: String,
        segments: Vec<SegmentInfo>,
    },
    CommitResult {
        seq: u64,
        session_id: String,
        success: bool,
    },
    ShutdownResult {
        seq: u64,
    },
    Error {
        seq: u64,
        #[serde(skip_serializing_if = "Option::is_none")]
        session_id: Option<String>,
        error: String,
    },
}

/// Server state
struct Server {
    converter: Converter,
}

impl Server {
    fn new() -> Self {
        let dictionary = load_dictionary();
        let converter = Converter::new(dictionary);
        Self { converter }
    }

    fn handle_request(&self, request: Request) -> Response {
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

/// Read a length-prefixed message from stdin
fn read_message<R: Read>(reader: &mut R) -> io::Result<Option<String>> {
    let len = match reader.read_u32::<BigEndian>() {
        Ok(len) => len,
        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(e),
    };

    if len > MAX_MESSAGE_SIZE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Message too large: {} bytes", len),
        ));
    }

    let mut buf = vec![0u8; len as usize];
    reader.read_exact(&mut buf)?;

    String::from_utf8(buf)
        .map(Some)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Invalid UTF-8: {}", e)))
}

/// Write a length-prefixed message to stdout
fn write_message<W: Write>(writer: &mut W, msg: &str) -> io::Result<()> {
    let bytes = msg.as_bytes();
    writer.write_u32::<BigEndian>(bytes.len() as u32)?;
    writer.write_all(bytes)?;
    writer.flush()
}

fn main() -> io::Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = BufReader::new(stdin.lock());
    let mut writer = stdout.lock();

    eprintln!("azuki-server v{} started", env!("CARGO_PKG_VERSION"));

    let server = Server::new();

    loop {
        let msg = match read_message(&mut reader)? {
            Some(msg) => msg,
            None => {
                eprintln!("EOF received, shutting down");
                break;
            }
        };

        let response = match serde_json::from_str::<Request>(&msg) {
            Ok(request) => {
                let is_shutdown = matches!(request, Request::Shutdown { .. });
                let response = server.handle_request(request);
                if is_shutdown {
                    let response_json =
                        serde_json::to_string(&response).expect("Failed to serialize response");
                    write_message(&mut writer, &response_json)?;
                    eprintln!("Shutdown requested, exiting");
                    break;
                }
                response
            }
            Err(e) => Response::Error {
                seq: 0,
                session_id: None,
                error: format!("Failed to parse request: {}", e),
            },
        };

        let response_json = serde_json::to_string(&response).expect("Failed to serialize response");
        write_message(&mut writer, &response_json)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_read_write_message() {
        let mut buf = Vec::new();
        let msg = r#"{"type":"init","seq":1}"#;
        write_message(&mut buf, msg).unwrap();

        let mut cursor = Cursor::new(buf);
        let read_msg = read_message(&mut cursor).unwrap().unwrap();
        assert_eq!(read_msg, msg);
    }

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
