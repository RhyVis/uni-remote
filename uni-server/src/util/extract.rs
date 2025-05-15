use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::element::{LoadedType, sc::SugarCubeInfo};

use super::AppState;

pub trait ExtractInfo {
    fn extract_info(&self, id: &str) -> Result<&LoadedType, Response>;
    fn extract_sc_info(&self, id: &str) -> Result<&SugarCubeInfo, Response>;
}

impl ExtractInfo for AppState {
    fn extract_info(&self, id: &str) -> Result<&LoadedType, Response> {
        self.get(id).ok_or_else(|| {
            (StatusCode::NOT_FOUND, format!("Info ID {id} not found")).into_response()
        })
    }
    fn extract_sc_info(&self, id: &str) -> Result<&SugarCubeInfo, Response> {
        match self.get(id).ok_or_else(|| {
            (StatusCode::NOT_FOUND, format!("SC Info ID {id} not found")).into_response()
        })? {
            LoadedType::Plain { .. } => Err((
                StatusCode::NOT_FOUND,
                format!("Info Id {id} found, but not SC type!"),
            )
                .into_response()),
            LoadedType::SugarCube { info } => Ok(info),
        }
    }
}
