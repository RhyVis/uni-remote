use std::sync::Arc;

use axum::{
    http::{
        header::{CACHE_CONTROL, CONTENT_TYPE, ETAG}, HeaderMap,
        StatusCode,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use lazy_static::lazy_static;

use crate::{
    constants::CACHE_HEADER,
    util::{
        etag::{etag_check, etag_hash},
        AppState,
    },
};

mod api;
mod asset;
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
        .nest("/api", api::routes())
        .merge(asset::routes())
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
