//! Length-prefixed JSON protocol for stdio communication

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::io::{self, Read, Write};

/// Maximum message size (4MB)
pub const MAX_MESSAGE_SIZE: u32 = 4 * 1024 * 1024;

/// Read a length-prefixed message from a reader
///
/// Message format: [u32 big-endian length][JSON bytes]
/// Returns None on EOF.
pub fn read_message<R: Read>(reader: &mut R) -> io::Result<Option<String>> {
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

/// Write a length-prefixed message to a writer
///
/// Message format: [u32 big-endian length][JSON bytes]
pub fn write_message<W: Write>(writer: &mut W, msg: &str) -> io::Result<()> {
    let bytes = msg.as_bytes();
    writer.write_u32::<BigEndian>(bytes.len() as u32)?;
    writer.write_all(bytes)?;
    writer.flush()
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
    fn test_read_eof() {
        let mut cursor = Cursor::new(Vec::new());
        let result = read_message(&mut cursor).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_message_too_large() {
        let mut buf = Vec::new();
        buf.write_u32::<BigEndian>(MAX_MESSAGE_SIZE + 1).unwrap();

        let mut cursor = Cursor::new(buf);
        let result = read_message(&mut cursor);
        assert!(result.is_err());
    }
}
