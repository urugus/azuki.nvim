//! azuki-server: Japanese input method conversion server
//!
//! Communicates via stdio using length-prefixed JSON protocol.

mod config;
mod converter;
mod dictionary;
mod handler;
mod message;
mod protocol;

use handler::Server;
use message::{extract_seq, Request, Response};
use protocol::{read_message, write_message};
use std::io::{self, BufReader};

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
            Err(e) => {
                let seq = extract_seq(&msg).unwrap_or(0);
                Response::Error {
                    seq,
                    session_id: None,
                    error: format!("Failed to parse request: {}", e),
                }
            }
        };

        let response_json = serde_json::to_string(&response).expect("Failed to serialize response");
        write_message(&mut writer, &response_json)?;
    }

    Ok(())
}
