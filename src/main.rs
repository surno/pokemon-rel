mod common;
mod config;
mod coordinator;
mod emulator;
mod error;
mod pipeline;

use crate::config::Configuration;
use crate::coordinator::CoordinatorBuilder;
use crate::error::AppError;
use crate::pipeline::orchestration::processing_pipeline::ProcessingPipeline;
use crate::pipeline::orchestration::step::scene_analyzer::SceneAnalyzer;
use tokio::time::Duration;
use tracing::Level;

fn init_logging() {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();
}

#[tokio::main]
async fn main() -> Result<(), AppError> {
    let coordinator = CoordinatorBuilder::new(Configuration::default())
        .rom_path("tests/roms/Super Mario Bros. 3 (USA, Europe) (Rev 1).nes".to_string())
        .frame_buffer_size(10)
        .action_buffer_size(10)
        .enable_metrics(true)
        .pipeline(
            ProcessingPipeline::builder()
                .add_analyzer(Box::new(SceneAnalyzer::new()))
                .build(),
        )
        .build()
        .expect("Failed to build coordinator");
    tokio::time::sleep(Duration::from_secs(30)).await;
    coordinator.stop();
    Ok(())
}
