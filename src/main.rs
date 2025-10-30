mod common;
mod config;
mod emulator;
mod error;

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
    Ok(())
}
