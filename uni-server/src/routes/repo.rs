use std::{fs, sync::Arc};

use axum::{
    extract::{Path, State},
    http::{
        header::{CACHE_CONTROL, CONTENT_TYPE, ETAG, IF_NONE_MATCH}, HeaderMap,
        StatusCode,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use lazy_static::lazy_static;
use tracing::{error, info};

use crate::{
    constants::{CACHE_HEADER, SSI_MOD_ID},
    util::{
        etag::{etag_check, etag_hash},
        extract::ExtractInfo,
        AppState,
    },
};

pub(super) fn routes() -> Router<Arc<AppState>> {
    Router::new().route(
        "/sc/mod/{manage_id}/{mod_id}/{mod_sub_id}",
        get(handle_sc_mods),
    )
}

const SSI_MOD_INTERNAL: &[u8] = include_bytes!("../../../resources/save-sync-integration.mod.zip");
lazy_static! {
    static ref SAVE_SYNC_INTEGRATION_ETAG: String = etag_hash(SSI_MOD_INTERNAL);
}

async fn handle_sc_mods(
    Path((manage_id, mod_id, mod_sub_id)): Path<(String, String, String)>,
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let game_info = match state.extract_sc_info(&manage_id) {
        Ok(info) => info,
        Err(res) => return res,
    };

    if !game_info.use_mods {
        return (
            StatusCode::BAD_REQUEST,
            format!("Game ID {manage_id} does not use mods"),
        )
            .into_response();
    }

    if mod_id == SSI_MOD_ID {
        info!("Responding to SSI Mod Request");

        if let Some(if_not_match) = headers.get(IF_NONE_MATCH) {
            if let Ok(cli_tag) = if_not_match.to_str() {
                if cli_tag == SAVE_SYNC_INTEGRATION_ETAG.as_str() {
                    return (
                        StatusCode::NOT_MODIFIED,
                        [
                            (CACHE_CONTROL, CACHE_HEADER),
                            (ETAG, SAVE_SYNC_INTEGRATION_ETAG.as_str()),
                        ],
                    )
                        .into_response();
                }
            }
        }

        return (
            StatusCode::OK,
            [
                (CONTENT_TYPE, "application/zip"),
                (CACHE_CONTROL, CACHE_HEADER),
                (ETAG, &SAVE_SYNC_INTEGRATION_ETAG),
            ],
            SSI_MOD_INTERNAL,
        )
            .into_response();
    }

    let mod_data = match game_info.get_mod(&mod_id, &mod_sub_id) {
        Some(path) => match fs::read(path) {
            Ok(data) => data,
            Err(err) => {
                error!("Failed to read mod file: {err}");
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to read mod file: {err}"),
                )
                    .into_response();
            }
        },
        None => {
            return (
                StatusCode::NOT_FOUND,
                format!("Mod ID {mod_id}:{mod_sub_id} not found"),
            )
                .into_response();
        }
    };
    info!("Responding to Mod ID: {mod_id}:{mod_sub_id}");

    if let Some(resp) = etag_check(&mod_data, &headers) {
        return resp;
    }

    let etag_val = etag_hash(&mod_data);
    (
        StatusCode::OK,
        [
            (CONTENT_TYPE, "application/zip"),
            (CACHE_CONTROL, CACHE_HEADER),
            (ETAG, etag_val.as_str()),
        ],
        mod_data,
    )
        .into_response()
}
