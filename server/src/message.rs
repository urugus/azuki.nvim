//! Request and Response message types for the azuki protocol

use crate::converter::Segment;
use serde::{Deserialize, Serialize};

/// Request types from the client
/// Fields marked with allow(dead_code) will be used in future phases
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[allow(dead_code)]
pub enum Request {
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
pub struct SegmentInput {
    pub reading: String,
    pub start: usize,
    pub length: usize,
    pub candidates: Vec<String>,
}

/// Options for conversion (will be used in future phases)
#[derive(Debug, Deserialize, Default)]
#[allow(dead_code)]
pub struct ConvertOptions {
    #[serde(default)]
    pub live: bool,
}

/// Segment info for response
#[derive(Debug, Serialize)]
pub struct SegmentInfo {
    pub reading: String,
    pub start: usize,
    pub length: usize,
    pub candidates: Vec<String>,
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

/// Response types to the client
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Response {
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

/// Extract seq from raw JSON string (for error handling when parse fails)
pub fn extract_seq(json: &str) -> Option<u64> {
    let value: serde_json::Value = serde_json::from_str(json).ok()?;
    value.get("seq")?.as_u64()
}
