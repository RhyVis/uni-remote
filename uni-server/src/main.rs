use tracing::info;
use util::config::config_ref;

#[allow(dead_code)]
mod element;
#[allow(dead_code)]
mod util;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    info!("Loading config...");
    let _ = config_ref();
}
