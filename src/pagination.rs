use base64::engine::general_purpose::STANDARD;
use base64::{DecodeError, engine::Engine};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const DEFAULT_PAGE_SIZE: usize = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PaginationMode {
    Default,
    Callers,
    Callees,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorData {
    pub mode: PaginationMode,
    pub offset: usize,
}

#[derive(Debug, Error)]
pub enum PaginationError {
    #[error("Invalid cursor: {0}")]
    InvalidCursor(String),
}

impl From<DecodeError> for PaginationError {
    fn from(err: DecodeError) -> Self {
        PaginationError::InvalidCursor(format!("Base64 decode error: {}", err))
    }
}

impl From<serde_json::Error> for PaginationError {
    fn from(err: serde_json::Error) -> Self {
        PaginationError::InvalidCursor(format!("JSON parse error: {}", err))
    }
}

pub fn encode_cursor(data: &CursorData) -> Result<String, PaginationError> {
    let json = serde_json::to_string(data)?;
    Ok(STANDARD.encode(json))
}

pub fn decode_cursor(cursor: &str) -> Result<CursorData, PaginationError> {
    let decoded = STANDARD.decode(cursor)?;
    let json_str = String::from_utf8(decoded)
        .map_err(|e| PaginationError::InvalidCursor(format!("UTF-8 decode error: {}", e)))?;
    Ok(serde_json::from_str(&json_str)?)
}

#[derive(Debug, Clone)]
pub struct PaginationResult<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<String>,
    pub total: usize,
}

pub fn paginate_slice<T: Clone>(
    items: &[T],
    offset: usize,
    page_size: usize,
    mode: PaginationMode,
) -> Result<PaginationResult<T>, PaginationError> {
    let total = items.len();

    if offset >= total {
        return Ok(PaginationResult {
            items: vec![],
            next_cursor: None,
            total,
        });
    }

    let end = std::cmp::min(offset + page_size, total);
    let page_items = items[offset..end].to_vec();

    let next_cursor = if end < total {
        let cursor_data = CursorData { mode, offset: end };
        Some(encode_cursor(&cursor_data)?)
    } else {
        None
    };

    Ok(PaginationResult {
        items: page_items,
        next_cursor,
        total,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cursor_encode_decode_roundtrip() {
        let original = CursorData {
            mode: PaginationMode::Default,
            offset: 42,
        };

        let encoded = encode_cursor(&original).expect("encode failed");
        let decoded = decode_cursor(&encoded).expect("decode failed");

        assert_eq!(decoded.mode, original.mode);
        assert_eq!(decoded.offset, original.offset);
    }

    #[test]
    fn test_pagination_mode_wire_format() {
        let cursor_data = CursorData {
            mode: PaginationMode::Callers,
            offset: 0,
        };

        let encoded = encode_cursor(&cursor_data).expect("encode failed");
        let decoded = decode_cursor(&encoded).expect("decode failed");

        assert_eq!(decoded.mode, PaginationMode::Callers);

        let json_str = serde_json::to_string(&cursor_data).expect("serialize failed");
        assert!(
            json_str.contains("\"mode\":\"callers\""),
            "expected lowercase 'callers' in JSON, got: {}",
            json_str
        );
    }

    #[test]
    fn test_paginate_slice_middle_page() {
        let items: Vec<i32> = (0..250).collect();
        let result =
            paginate_slice(&items, 100, 100, PaginationMode::Default).expect("paginate failed");

        assert_eq!(result.items.len(), 100);
        assert_eq!(result.items[0], 100);
        assert_eq!(result.items[99], 199);
        assert!(result.next_cursor.is_some());
        assert_eq!(result.total, 250);
    }

    #[test]
    fn test_paginate_slice_empty_and_beyond() {
        let empty: Vec<i32> = vec![];
        let result =
            paginate_slice(&empty, 0, 100, PaginationMode::Default).expect("paginate failed");
        assert_eq!(result.items.len(), 0);
        assert!(result.next_cursor.is_none());
        assert_eq!(result.total, 0);

        let items: Vec<i32> = (0..50).collect();
        let result =
            paginate_slice(&items, 100, 100, PaginationMode::Default).expect("paginate failed");
        assert_eq!(result.items.len(), 0);
        assert!(result.next_cursor.is_none());
        assert_eq!(result.total, 50);
    }

    #[test]
    fn test_paginate_slice_exact_boundary() {
        let items: Vec<i32> = (0..200).collect();
        let result =
            paginate_slice(&items, 100, 100, PaginationMode::Default).expect("paginate failed");

        assert_eq!(result.items.len(), 100);
        assert_eq!(result.items[0], 100);
        assert!(result.next_cursor.is_none());
        assert_eq!(result.total, 200);
    }

    #[test]
    fn test_invalid_cursor_error() {
        let result = decode_cursor("not-valid-base64!!!");
        assert!(result.is_err());
        match result {
            Err(PaginationError::InvalidCursor(msg)) => {
                assert!(msg.contains("Base64") || msg.contains("decode"));
            }
            _ => panic!("Expected InvalidCursor error"),
        }
    }
}
