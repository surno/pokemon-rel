mod app;
mod common;
mod config;
mod coordinator;
mod emulator;
mod error;
mod pipeline;

use crate::app::PokeBot;
use crate::config::Configuration;
use crate::coordinator::CoordinatorBuilder;
use crate::error::AppError;
use crate::pipeline::orchestration::processing_pipeline::ProcessingPipeline;
use crate::pipeline::orchestration::step::scene_analyzer::SceneAnalyzer;
use tracing::Level;

fn init_logging() {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();
}

#[tokio::main]
async fn main() -> Result<(), AppError> {
    init_logging();
    let mut coordinator = CoordinatorBuilder::new(Configuration::default())
        .rom_path("/Users/tony/Projects/pokemon-shiny/POKEMON_B_IRBO01_00.nds".to_string())
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

    let result = eframe::run_native(
        "PokeBot",
        eframe::NativeOptions::default(),
        Box::new(|cc| Ok(Box::new(PokeBot::new(cc, coordinator.frame_rx().unwrap())))),
    );

    if let Err(e) = result {
        tracing::error!("Error running PokeBot: {}", e);
        return Err(AppError::Eframe(e));
    }

    Ok(())
}
