use axum::{
    http::{
        HeaderMap, StatusCode,
        header::{CACHE_CONTROL, ETAG, IF_NONE_MATCH},
    },
    response::{IntoResponse, Response},
};
use xxhash_rust::xxh3::xxh3_64;

use crate::constants::CACHE_HEADER;

pub fn etag_hash(content: &[u8]) -> String {
    format!("\"{}\"", xxh3_64(content))
}

pub fn etag_check(content: &[u8], headers: &HeaderMap) -> Option<Response> {
    let etag_val = etag_hash(content);

    if let Some(if_none_match) = headers.get(IF_NONE_MATCH) {
        if let Ok(cli_tag) = if_none_match.to_str() {
            if cli_tag == etag_val {
                return Some(
                    (
                        StatusCode::NOT_MODIFIED,
                        [(CACHE_CONTROL, CACHE_HEADER), (ETAG, etag_val.as_str())],
                    )
                        .into_response(),
                );
            }
        }
    }

    None
}
