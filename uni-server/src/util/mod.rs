use std::{env::current_dir, path::PathBuf};
use tracing::{error, error_span};

use crate::element::{LoadedMapping, LoadedType};

pub(crate) mod config;
pub(crate) mod etag;
pub(crate) mod extract;
pub(crate) mod mfs;
pub(crate) mod path_ext;

#[derive(Debug)]
pub struct AppState(LoadedMapping);

impl AppState {
    pub fn new(mapping: LoadedMapping) -> Self {
        Self(mapping)
    }

    pub fn get(&self, id: &str) -> Option<&LoadedType> {
        self.0.get(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &LoadedType)> {
        self.0.iter()
    }
}

/// Current working directory, absolute path
pub fn cd() -> PathBuf {
    let base = current_dir().unwrap_or_else(|e| {
        let span = error_span!("util::cd");
        span.in_scope(|| {
            error!("Failed to get current directory: {}", e);
            error!("Using default path: .");
        });
        PathBuf::from(".")
    });

    #[cfg(debug_assertions)]
    {
        return base.join(".run");
    }

    #[cfg(not(debug_assertions))]
    {
        return base;
    }
}

/// Change directory, relative to current working directory
///
/// Based on [cd]
pub fn cd_in(path: &str) -> PathBuf {
    cd().join(path)
}
