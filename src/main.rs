mod app;
mod config;
mod emulator;
mod error;
mod intake;
mod network;
mod pipeline;

use crate::app::multiclient_app::MultiClientApp;
use crate::config::Settings;
use crate::error::AppError;
use tracing::Level;

fn init_logging() {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();
}

#[tokio::main]
async fn main() -> Result<(), AppError> {
    let settings = Settings::new().map_err(|e| AppError::Config(e.to_string()))?;
    init_logging();
    MultiClientApp::start_gui(&settings);
    Ok(())
}
