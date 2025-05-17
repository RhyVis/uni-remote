use crate::element::LoadedType;
use crate::util::AppState;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;
use std::sync::Arc;

pub(super) fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/list-all", get(api_list_playable))
}

async fn api_list_playable(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    #[derive(Debug, Serialize)]
    #[serde(rename_all = "camelCase")]
    struct PlayableInfo {
        id: String,
        name: Option<String>,
        manage: PlayableType,
    }

    #[derive(Debug, Serialize)]
    enum PlayableType {
        Plain(String),
        SugarCube(Vec<SugarCubeLabel>),
    }

    #[derive(Debug, Serialize)]
    struct SugarCubeLabel {
        id: String,
        name: Option<String>,
        index: String,
        layers: Vec<String>,
        mods: Option<Vec<(String, String)>>,
    }

    let mut list = state
        .iter()
        .map(|(id, lt)| {
            let (manage, name) = match lt {
                LoadedType::Plain { original_ref, .. } => (
                    PlayableType::Plain("0".to_string()),
                    original_ref.name.clone(),
                ),
                LoadedType::SugarCube { info, .. } => (
                    PlayableType::SugarCube(
                        info.instances
                            .iter()
                            .map(|(key, instance)| SugarCubeLabel {
                                id: key.to_string(),
                                name: instance.name.clone(),
                                index: instance.original_conf.index.to_string(),
                                layers: instance.original_conf.layers.clone(),
                                mods: if info.use_mods {
                                    Some(instance.original_conf.mods.clone())
                                } else {
                                    None
                                },
                            })
                            .collect(),
                    ),
                    info.name.clone(),
                ),
            };

            PlayableInfo {
                id: id.to_string(),
                name,
                manage,
            }
        })
        .collect::<Vec<_>>();
    list.sort_by_key(|i| i.id.clone());

    Json(list).into_response()
}
