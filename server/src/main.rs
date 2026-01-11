use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use serde::{Deserialize, Serialize};
use std::io::{self, BufReader, Read, Write};

/// Maximum message size (4MB)
const MAX_MESSAGE_SIZE: u32 = 4 * 1024 * 1024;

// Request types
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
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
}

#[derive(Debug, Deserialize, Default)]
struct ConvertOptions {
    #[serde(default)]
    live: bool,
}

// Response types
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Response {
    InitResult {
        seq: u64,
        session_id: String,
        version: String,
    },
    ConvertResult {
        seq: u64,
        session_id: String,
        candidates: Vec<String>,
        selected_index: usize,
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

    String::from_utf8(buf).map(Some).map_err(|e| {
        io::Error::new(io::ErrorKind::InvalidData, format!("Invalid UTF-8: {}", e))
    })
}

/// Write a length-prefixed message to stdout
fn write_message<W: Write>(writer: &mut W, msg: &str) -> io::Result<()> {
    let bytes = msg.as_bytes();
    writer.write_u32::<BigEndian>(bytes.len() as u32)?;
    writer.write_all(bytes)?;
    writer.flush()
}

/// Simple hiragana pass-through conversion (placeholder)
/// In the future, this will use a proper conversion backend
fn convert_hiragana(reading: &str) -> Vec<String> {
    // For Phase 1, just return the reading as-is
    // This will be replaced with actual conversion logic
    vec![reading.to_string()]
}

fn handle_request(request: Request) -> Response {
    match request {
        Request::Init { seq, session_id } => {
            let session_id = session_id.unwrap_or_else(|| {
                format!("session_{}", std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis())
            });
            Response::InitResult {
                seq,
                session_id,
                version: env!("CARGO_PKG_VERSION").to_string(),
            }
        }
        Request::Convert {
            seq,
            session_id,
            reading,
            cursor: _,
            options: _,
        } => {
            let candidates = convert_hiragana(&reading);
            Response::ConvertResult {
                seq,
                session_id,
                candidates,
                selected_index: 0,
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
    }
}

fn main() -> io::Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = BufReader::new(stdin.lock());
    let mut writer = stdout.lock();

    eprintln!("azuki-server v{} started", env!("CARGO_PKG_VERSION"));

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
                let response = handle_request(request);
                if is_shutdown {
                    let response_json = serde_json::to_string(&response)
                        .expect("Failed to serialize response");
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

        let response_json =
            serde_json::to_string(&response).expect("Failed to serialize response");
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
        let json = r#"{"type":"init","seq":1}"#;
        let request: Request = serde_json::from_str(json).unwrap();
        let response = handle_request(request);
        match response {
            Response::InitResult { seq, version, .. } => {
                assert_eq!(seq, 1);
                assert!(!version.is_empty());
            }
            _ => panic!("Expected InitResult"),
        }
    }

    #[test]
    fn test_convert_request() {
        let json = r#"{"type":"convert","seq":42,"session_id":"abc","reading":"きょうは"}"#;
        let request: Request = serde_json::from_str(json).unwrap();
        let response = handle_request(request);
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
            }
            _ => panic!("Expected ConvertResult"),
        }
    }

    #[test]
    fn test_shutdown_request() {
        let json = r#"{"type":"shutdown","seq":99}"#;
        let request: Request = serde_json::from_str(json).unwrap();
        let response = handle_request(request);
        match response {
            Response::ShutdownResult { seq } => {
                assert_eq!(seq, 99);
            }
            _ => panic!("Expected ShutdownResult"),
        }
    }
}
