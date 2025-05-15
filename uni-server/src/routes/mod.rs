use std::sync::Arc;

use axum::{
    Router,
    http::{
        HeaderMap, StatusCode,
        header::{CACHE_CONTROL, CONTENT_TYPE, ETAG},
    },
    response::IntoResponse,
    routing::get,
};
use lazy_static::lazy_static;

use crate::{
    constants::CACHE_HEADER,
    util::{
        AppState,
        etag::{etag_check, etag_hash},
    },
};

mod play;
mod repo;

const ICON: &[u8] = include_bytes!("../../../resources/favicon.ico");

lazy_static! {
    static ref ICON_ETAG: String = etag_hash(ICON);
}

pub fn main_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/favicon.ico", get(favicon))
        .nest("/play", play::routes())
        .nest("/repo", repo::routes())
}

async fn favicon(headers: HeaderMap) -> impl IntoResponse {
    if let Some(res) = etag_check(ICON, &headers) {
        return res;
    }

    (
        StatusCode::OK,
        [
            (CONTENT_TYPE, "image/x-icon"),
            (CACHE_CONTROL, CACHE_HEADER),
            (ETAG, &ICON_ETAG),
        ],
        ICON,
    )
        .into_response()
}
