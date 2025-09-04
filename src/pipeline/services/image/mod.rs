pub mod analysis; // New modular scene analysis architecture
pub mod color_analysis_service;

pub use analysis::{SceneAnalysisConfig, SceneAnalysisOrchestrator};
pub use color_analysis_service::*;
