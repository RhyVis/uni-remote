use std::sync::Arc;

use axum::{
    Router,
    extract::Path,
    http::{StatusCode, header::CONTENT_TYPE},
    response::IntoResponse,
    routing::get,
};
use rust_embed::Embed;
use tracing::info;

use crate::util::AppState;

#[derive(Embed)]
#[folder = "../uni-page/dist/"]
struct Assets;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/{*path}", get(static_handler))
        .route("/", get(index_handler))
}

async fn static_handler(Path(path): Path<String>) -> impl IntoResponse {
    let path = &path;
    match Assets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            info!("Serving asset: {path}, mime: {mime}");
            (
                StatusCode::OK,
                [(CONTENT_TYPE, mime.as_ref())],
                content.data,
            )
                .into_response()
        }
        None => (StatusCode::NOT_FOUND, format!("File not found: {path}")).into_response(),
    }
}

async fn index_handler() -> impl IntoResponse {
    match Assets::get("index.html") {
        Some(content) => {
            info!("Serving index.html");
            (StatusCode::OK, [(CONTENT_TYPE, "text/html")], content.data).into_response()
        }
        None => (StatusCode::NOT_FOUND, "File not found: index.html").into_response(),
    }
}
