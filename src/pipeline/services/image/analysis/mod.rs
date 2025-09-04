pub mod analyzers;
pub mod config;
pub mod core;
pub mod detectors;
pub mod orchestrator;
pub mod pipeline;

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
