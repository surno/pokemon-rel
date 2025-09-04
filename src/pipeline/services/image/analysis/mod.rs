pub mod analyzers;
pub mod bag_menu_detector;
pub mod config;
pub mod core;
pub mod detectors;
pub mod menu_cursor_detector;
pub mod orchestrator;
pub mod pipeline;
pub mod pokemon_detector;
pub mod shiny_detector;

pub use analyzers::{
    EnvironmentDetector, HPBarDetector, LocationDetector, MenuDetector, TextDetector,
};
pub use config::{ColorThresholds, DetectorType, SceneAnalysisConfig};
pub use core::{
    DetectionContext, DetectionResult, DetectionSignal, GameStateAnalyzer, ImageRegion,
    SceneDetector, VisualDetector,
};
pub use detectors::{
    BattleSceneDetector, IntroSceneDetector, MenuSceneDetector, OverworldSceneDetector,
};
pub use orchestrator::SceneAnalysisOrchestrator;
pub use pipeline::DetectionPipeline;
