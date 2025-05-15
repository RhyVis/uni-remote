use std::{fs, path::PathBuf, sync::Arc};

use axum::{
    Json, Router,
    extract::{Path, State},
    http::{
        HeaderMap, StatusCode,
        header::{CACHE_CONTROL, CONTENT_TYPE, ETAG},
    },
    response::{Html, IntoResponse, Response},
    routing::get,
};
use tracing::{error, warn};

use crate::{
    constants::CACHE_HEADER,
    element::LoadedType,
    util::{
        AppState,
        etag::{etag_check, etag_hash},
        extract::ExtractInfo,
    },
};

pub(super) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/{manage_id}/{instance_id}/index-path",
            get(handle_play_index),
        )
        .route(
            "/{manage_id}/{instance_id}/modList.json",
            get(handle_mod_list),
        )
        .route(
            "/{manage_id}/{instance_id}/{*other_path}",
            get(handle_other_path),
        )
}

async fn handle_play_index(
    Path((manage_id, instance_id)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let info = match state.extract_info(&manage_id) {
        Ok(info) => info,
        Err(resp) => return resp,
    };

    fn read_html(path: &PathBuf, headers: &HeaderMap) -> Response {
        match fs::read(path) {
            Ok(html) => {
                if let Some(resp) = etag_check(&html, &headers) {
                    return resp;
                }

                let etag_val = etag_hash(&html);
                return (
                    StatusCode::OK,
                    [
                        (CONTENT_TYPE, "text/html; charset=utf-8"),
                        (CACHE_CONTROL, CACHE_HEADER),
                        (ETAG, etag_val.as_str()),
                    ],
                    Html(String::from_utf8_lossy(&html).to_string()),
                )
                    .into_response();
            }
            Err(err) => {
                error!("Failed to read html file: {err}");
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to read enter file: {err}"),
                )
                    .into_response();
            }
        }
    }

    match info {
        LoadedType::Plain { enter_path, .. } => read_html(&enter_path, &headers),
        LoadedType::SugarCube { info } => {
            let instance = match info.get_instance(&instance_id) {
                Some(instance) => instance,
                None => {
                    warn!("Instance ID {instance_id} in {manage_id} not found");
                    return (
                        StatusCode::NOT_FOUND,
                        format!("Instance ID {instance_id} in {manage_id} not found"),
                    )
                        .into_response();
                }
            };
            read_html(&instance.index_path, &headers)
        }
    }
}

async fn handle_mod_list(
    Path((manage_id, instance_id)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let info = match state.extract_sc_info(&manage_id) {
        Ok(info) => info,
        Err(resp) => {
            warn!("Failed to extract SC info for {manage_id}: {instance_id}");
            return resp;
        }
    };

    match info.generate_mod_list(&instance_id, &manage_id) {
        Ok(mod_list) => Json(mod_list).into_response(),
        Err(resp) => {
            warn!("Failed to generate mod list: {resp:?}");
            resp
        }
    }
}

async fn handle_other_path(
    Path((manage_id, instance_id, other_path)): Path<(String, String, String)>,
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let loaded_type = match state.extract_info(&manage_id) {
        Ok(info) => info,
        Err(resp) => return resp,
    };

    fn read_file(path: &PathBuf, headers: &HeaderMap, manage_id: &str) -> Response {
        if !path.exists() || path.is_dir() {
            warn!("File not found for '{}': {}", manage_id, path.display());
            return (
                StatusCode::NOT_FOUND,
                format!("File not found for '{}': {}", manage_id, path.display()),
            )
                .into_response();
        }

        match fs::metadata(&path) {
            Ok(metadata) => {
                const MAX_FILE_SIZE: u64 = 64 * 1024 * 1024;
                if metadata.len() > MAX_FILE_SIZE {
                    error!(
                        "File size exceeds limit: {} bytes, path: {:?}",
                        metadata.len(),
                        path
                    );
                    return (
                        StatusCode::BAD_REQUEST,
                        format!(
                            "The file size exceeds the limit of {} MB",
                            MAX_FILE_SIZE / 1024 / 1024
                        ),
                    )
                        .into_response();
                }
            }
            Err(err) => {
                error!("Failed to get metadata: {}, path: {:?}", err, path);
            }
        }

        let content = match fs::read(path) {
            Ok(content) => content,
            Err(err) => {
                error!("Failed to read file: {}, path: {:?}", err, path);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to read file for '{manage_id}': {err}"),
                )
                    .into_response();
            }
        };
        if let Some(resp) = etag_check(&content, headers) {
            return resp;
        }

        let mime = mime_guess::from_path(&path).first_or_octet_stream();
        let etag_val = etag_hash(&content);
        (
            StatusCode::OK,
            [
                (CONTENT_TYPE, mime.as_ref()),
                (CACHE_CONTROL, CACHE_HEADER),
                (ETAG, etag_val.as_str()),
            ],
            content,
        )
            .into_response()
    }

    match loaded_type {
        LoadedType::Plain { root_path, .. } => {
            let mut actual_path = root_path.clone();

            for component in other_path.split('/') {
                if component.is_empty() || component == "." {
                    continue;
                }
                if component == ".." {
                    warn!("Path traversal detected in {manage_id}");
                    return (
                        StatusCode::FORBIDDEN,
                        "Path traversal is not allowed".to_string(),
                    )
                        .into_response();
                }
                actual_path.push(component);
            }

            read_file(&actual_path, &headers, &manage_id)
        }
        LoadedType::SugarCube { info } => {
            let instance = match info.get_instance(&instance_id) {
                Some(instance) => instance,
                None => {
                    warn!("Instance ID {instance_id} in {manage_id} not found");
                    return (
                        StatusCode::NOT_FOUND,
                        format!("Instance ID {instance_id} in {manage_id} not found"),
                    )
                        .into_response();
                }
            };
            let actual_node = match instance.layer_merged.get(&other_path) {
                Some(path) => path,
                None => {
                    warn!("Path '{other_path}' not found in instance {instance_id}");
                    return (
                        StatusCode::NOT_FOUND,
                        format!("Path '{other_path}' not found in instance {instance_id}"),
                    )
                        .into_response();
                }
            };

            let (content, file_name) = match actual_node.resolve() {
                Some(content) => content,
                None => {
                    warn!("Failed to resolve path '{other_path}' in instance {instance_id}");
                    return (
                        StatusCode::NOT_FOUND,
                        format!("Failed to resolve path '{other_path}' in instance {instance_id}"),
                    )
                        .into_response();
                }
            };
            if let Some(resp) = etag_check(&content, &headers) {
                return resp;
            }

            let mime = mime_guess::from_path(file_name).first_or_octet_stream();
            let etag_val = etag_hash(&content);
            (
                StatusCode::OK,
                [
                    (CONTENT_TYPE, mime.as_ref()),
                    (CACHE_CONTROL, CACHE_HEADER),
                    (ETAG, etag_val.as_str()),
                ],
                content,
            )
                .into_response()
        }
    }
}
