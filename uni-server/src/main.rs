use std::sync::Arc;

use anyhow::Result;
use axum::Router;
use element::load_data_dir;
use routes::main_routes;
use tokio::net::TcpListener;
use tracing::info;
use util::{
    AppState,
    config::{ReadConfig, config_ref},
};

mod constants;
mod element;
mod routes;
mod util;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    info!("Loading config...");
    let loaded_mapping = load_data_dir()?;

    let port = config_ref().port();
    let addr = format!("0.0.0.0:{port}");

    let app = Router::new()
        .merge(main_routes())
        .with_state(Arc::new(AppState::new(loaded_mapping)));
    let listener = TcpListener::bind(&addr).await?;
    info!("Listening on {addr}");

    axum::serve(listener, app).await?;

    Ok(())
}
