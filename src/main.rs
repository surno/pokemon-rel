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
use crate::emulator::emulator_client;
use crate::error::AppError;
use crate::pipeline::orchestration::processing_pipeline::ProcessingPipeline;
use crate::pipeline::orchestration::step::scene_analyzer::SceneAnalyzer;
use tracing::Level;

fn init_logging() {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();
}

fn main() -> Result<(), AppError> {
    init_logging();
    
    let configuration = Configuration::default();
    let rom_path = "/Users/tony/Projects/pokemon-shiny/POKEMON_B_IRBO01_00.nds".to_string();
    
    // Initialize emulator on main thread (required for Metal thread safety on macOS)
    // This must happen before creating the tokio runtime to ensure we're on the actual macOS main thread
    tracing::info!("Initializing emulator on main thread before tokio runtime");
    let desmume = emulator_client::initialize_emulator(
        rom_path.clone(),
        configuration.renderer.clone(),
    )?;
    
    // Create tokio runtime after emulator initialization
    // The runtime will be used for async coordinator operations
    let rt = tokio::runtime::Runtime::new()
        .map_err(|_e| AppError::Unknown)?;
    
    // Create coordinator with pre-initialized emulator
    // Use block_on to create the coordinator within the tokio runtime context
    let mut coordinator = rt.block_on(async {
        CoordinatorBuilder::new(configuration)
            .rom_path(rom_path)
            .frame_buffer_size(10)
            .action_buffer_size(10)
            .enable_metrics(true)
            .pipeline(
                ProcessingPipeline::builder()
                    .add_analyzer(Box::new(SceneAnalyzer::new()))
                    .build(),
            )
            .desmume(desmume)  // Pass pre-initialized emulator
            .build()
            .expect("Failed to build coordinator")
    });
    
    // Run UI on main thread (eframe::run_native runs on the actual macOS main thread)
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
