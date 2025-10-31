mod common;
mod config;
mod coordinator;
mod emulator;
mod error;
mod pipeline;

use crate::config::Configuration;
use crate::coordinator::CoordinatorBuilder;
use crate::error::AppError;
use tracing::Level;

fn init_logging() {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();
}

#[tokio::main]
async fn main() -> Result<(), AppError> {
    let configuration = Configuration::default();
    init_logging();
    Ok(())
}
