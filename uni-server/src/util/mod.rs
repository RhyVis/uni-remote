use std::{env::current_dir, path::PathBuf};
use tracing::{error, error_span};

pub(crate) mod config;
pub(crate) mod mfs;
pub(crate) mod path_ext;

/// Current working directory, absolute path
pub fn cd() -> PathBuf {
    current_dir().unwrap_or_else(|e| {
        let span = error_span!("util::cd");
        span.in_scope(|| {
            error!("Failed to get current directory: {}", e);
            error!("Using default path: .");
        });
        PathBuf::from(".")
    })
}

/// Change directory, relative to current working directory
///
/// Based on [cd]
pub fn cd_in(path: &str) -> PathBuf {
    cd().join(path)
}
