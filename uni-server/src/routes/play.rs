use axum::routing::post;
use axum::{
    extract::{Path, State}, http::{
        header::{CACHE_CONTROL, CONTENT_TYPE, ETAG}, HeaderMap,
        StatusCode,
    },
    response::{Html, IntoResponse, Response},
    routing::get,
    Json,
    Router,
};
use chrono::Local;
use serde::Deserialize;
use std::{fs, path::PathBuf, sync::Arc};
use tracing::{error, info, warn};

use crate::util::config::{config_ref, ReadConfig};
use crate::util::path_ext::PathHelper;
use crate::{
    constants::CACHE_HEADER,
    element::LoadedType,
    util::{
        etag::{etag_check, etag_hash},
        extract::ExtractInfo,
        AppState,
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
            "/{manage_id}/{instance_id}/save-sync/list",
            get(handle_save_list),
        )
        .route(
            "/{manage_id}/{instance_id}/save-sync/access",
            post(handle_save_upload),
        )
        .route(
            "/{manage_id}/{instance_id}/save-sync/access/{save_id}",
            get(handle_save_get).delete(handle_save_del),
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
                (
                    StatusCode::OK,
                    [
                        (CONTENT_TYPE, "text/html; charset=utf-8"),
                        (CACHE_CONTROL, CACHE_HEADER),
                        (ETAG, etag_val.as_str()),
                    ],
                    Html(String::from_utf8_lossy(&html).to_string()),
                )
                    .into_response()
            }
            Err(err) => {
                error!("Failed to read html file: {err}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to read enter file: {err}"),
                )
                    .into_response()
            }
        }
    }

    match info {
        LoadedType::Plain { enter_path, .. } => read_html(&enter_path, &headers),
        LoadedType::SugarCube { info, .. } => {
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
        LoadedType::SugarCube { info, .. } => {
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

fn to_save_path(manage_id: &str, instance_id: &str) -> PathBuf {
    let mut data_dir = config_ref().data_dir();
    data_dir.push(manage_id);
    data_dir.push(instance_id);
    data_dir.push("save");
    data_dir
}

fn check_save_func(
    manage_id: &str,
    instance_id: &str,
    state: &Arc<AppState>,
) -> Result<PathBuf, Response> {
    let info = match state.extract_sc_info(&manage_id) {
        Ok(info) => info,
        Err(resp) => {
            warn!("Failed to extract SC info for {manage_id}: {instance_id}");
            return Err(resp);
        }
    };
    if !info.use_save_sync_mod || !info.use_mods {
        return Err((
            StatusCode::NOT_FOUND,
            format!("Feature 'use_save_sync_mod' is not enabled for {manage_id}").to_string(),
        )
            .into_response());
    }
    if let Some(resp) = info.check_instance(&instance_id) {
        return Err(resp);
    }

    let path = to_save_path(manage_id, instance_id);
    if !path.exists() {
        match fs::create_dir_all(&path) {
            Ok(_) => Ok(path),
            Err(err) => {
                error!(
                    "Failed to create save directory on {}: {err}",
                    path.display()
                );
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to create save directory: {err}").to_string(),
                )
                    .into_response())
            }
        }
    } else {
        Ok(path)
    }
}

async fn handle_save_list(
    Path((manage_id, instance_id)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let save_path = match check_save_func(&manage_id, &instance_id, &state) {
        Ok(path) => path,
        Err(resp) => {
            return resp;
        }
    };

    match fs::read_dir(&save_path).map(|e| {
        e.into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension_eq("save"))
            .map(|e| {
                e.file_name()
                    .to_string_lossy()
                    .to_string()
                    .strip_suffix(".save")
                    .unwrap_or_default()
                    .to_string()
            })
            .filter(|e| !e.trim().is_empty())
            .collect::<Vec<_>>()
    }) {
        Ok(o) => Json(o).into_response(),
        Err(err) => {
            error!("Failed to read save directory: {err}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to read save directory: {err}"),
            )
                .into_response()
        }
    }
}

async fn handle_save_get(
    Path((manage_id, instance_id, save_id)): Path<(String, String, String)>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let save_path = match check_save_func(&manage_id, &instance_id, &state) {
        Ok(path) => path,
        Err(resp) => {
            return resp;
        }
    };

    let save_content = match fs::read_to_string(save_path.join(format!("{save_id}.save"))) {
        Ok(content) => content,
        Err(err) => {
            error!("Failed to read save file by ({manage_id}:{instance_id}:{save_id}): {err}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to read save file: {err}"),
            )
                .into_response();
        }
    };
    info!("Requested save file: {manage_id}:{instance_id}:{save_id}");
    save_content.into_response()
}

async fn handle_save_del(
    Path((manage_id, instance_id, save_id)): Path<(String, String, String)>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let save_path = match check_save_func(&manage_id, &instance_id, &state) {
        Ok(path) => path,
        Err(resp) => {
            return resp;
        }
    };

    let save_file = save_path.join(format!("{save_id}.save"));
    if save_file.exists() {
        if let Err(err) = fs::remove_file(save_file) {
            error!("Failed to delete save file: {err}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to delete save file: {err}"),
            )
                .into_response();
        };
    } else {
        let err_msg = format!(
            "Failed to find save file for deleting: {}",
            save_file.display()
        );
        warn!(err_msg);
        return err_msg.into_response();
    }

    info!("Deleted save file: {manage_id}:{instance_id}:{save_id}");
    format!("Successfully deleted {save_id}").into_response()
}

async fn handle_save_upload(
    Path((manage_id, instance_id)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
    Json(save_code): Json<SaveCode>,
) -> impl IntoResponse {
    let save_path = match check_save_func(&manage_id, &instance_id, &state) {
        Ok(path) => path,
        Err(resp) => {
            return resp;
        }
    };

    let timestamp = Local::now().format("%Y-%m-%d+%H-%M-%S").to_string();
    let file_name = format!("{}@{timestamp}.save", save_code.alias());

    let file_path = save_path.join(&file_name);
    if file_path.exists() {
        warn!("Save file already exists: {}", file_path.display());
    }

    match fs::write(&file_path, save_code.code()) {
        Ok(_) => {
            info!("Save file created: {}", file_path.display());
            StatusCode::NO_CONTENT.into_response()
        }
        Err(err) => {
            error!("Failed to write save file: {err}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to write save file: {err}"),
            )
                .into_response()
        }
    }
}

#[derive(Debug, Deserialize)]
struct SaveCode {
    code: String,
    alias: String,
}

impl SaveCode {
    pub fn code(&self) -> &str {
        self.code.as_str()
    }
    pub fn alias(&self) -> String {
        if self.alias.is_empty() {
            "anonymous".to_string()
        } else {
            self.alias.clone()
        }
    }
}
