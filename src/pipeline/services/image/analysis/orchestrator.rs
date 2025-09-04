/// Scene Analysis Orchestrator - replaces the monolithic SceneAnnotationService
use super::{
    analyzers::{EnvironmentDetector, HPBarDetector, LocationDetector, MenuDetector, TextDetector},
    config::SceneAnalysisConfig,
    core::{DetectionContext, DetectionResult, GameStateAnalyzer, SceneDetector},
    detectors::{
        BattleSceneDetector, IntroSceneDetector, MenuSceneDetector, OverworldSceneDetector,
        PokemonStateAnalyzer,
    },
    pipeline::DetectionPipeline,
};
use crate::{
    error::AppError,
    pipeline::types::{LocationType, StoryProgress},
    pipeline::{EnrichedFrame, Scene, State},
};
use image::DynamicImage;
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::Instant,
};
use tower::Service;
use tracing::{debug, info};

/// Main orchestrator for scene analysis - clean, focused, and configurable!
/// This replaces the monolithic SceneAnnotationService with elegant design patterns
pub struct SceneAnalysisOrchestrator {
    scene_detectors: Vec<Box<dyn SceneDetector>>,
    state_analyzer: Box<dyn GameStateAnalyzer>,
    detection_pipeline: DetectionPipeline,
    config: SceneAnalysisConfig,
}

impl SceneAnalysisOrchestrator {
    pub fn new(config: SceneAnalysisConfig) -> Result<Self, AppError> {
        config
            .validate()
            .map_err(|e| AppError::Client(format!("Invalid config: {}", e)))?;

        // Create scene detectors
        let scene_detectors: Vec<Box<dyn SceneDetector>> = vec![
            Box::new(BattleSceneDetector::new()),
            Box::new(MenuSceneDetector::new()),
            Box::new(OverworldSceneDetector::new()),
            Box::new(IntroSceneDetector::new()),
        ];

        // Create state analyzer
        let state_analyzer: Box<dyn GameStateAnalyzer> = Box::new(PokemonStateAnalyzer::new());

        // Create detection pipeline with enabled detectors
        let mut detection_pipeline = DetectionPipeline::new();

        // Add detectors based on configuration
        for detector_type in &config.enabled_detectors {
            match detector_type {
                super::config::DetectorType::HPBar => {
                    detection_pipeline =
                        detection_pipeline.add_detector(Box::new(HPBarDetector::new()));
                }
                super::config::DetectorType::BattleMenu | super::config::DetectorType::MainMenu => {
                    detection_pipeline =
                        detection_pipeline.add_detector(Box::new(MenuDetector::new()));
                }
                super::config::DetectorType::TextBlock | super::config::DetectorType::DialogBox => {
                    detection_pipeline =
                        detection_pipeline.add_detector(Box::new(TextDetector::new()));
                }
                super::config::DetectorType::PokemonCenter
                | super::config::DetectorType::Gym
                | super::config::DetectorType::Cave
                | super::config::DetectorType::City
                | super::config::DetectorType::Town
                | super::config::DetectorType::Route
                | super::config::DetectorType::Building => {
                    detection_pipeline =
                        detection_pipeline.add_detector(Box::new(LocationDetector::new()));
                }
                super::config::DetectorType::TallGrass
                | super::config::DetectorType::Water
                | super::config::DetectorType::Indoor => {
                    detection_pipeline =
                        detection_pipeline.add_detector(Box::new(EnvironmentDetector::new()));
                }
                _ => {
                    // Skip unsupported detector types for now
                }
            }
        }

        // Configure pipeline based on performance mode
        match config.performance_mode {
            super::config::PerformanceMode::Speed => {
                detection_pipeline = detection_pipeline
                    .with_early_termination(true)
                    .with_max_processing_time(5_000); // 5ms limit
            }
            super::config::PerformanceMode::Balanced => {
                detection_pipeline = detection_pipeline
                    .with_early_termination(true)
                    .with_max_processing_time(10_000); // 10ms limit
            }
            super::config::PerformanceMode::Accuracy => {
                detection_pipeline = detection_pipeline
                    .with_early_termination(false)
                    .with_max_processing_time(20_000); // 20ms limit
            }
        }

        Ok(Self {
            scene_detectors,
            state_analyzer,
            detection_pipeline,
            config,
        })
    }

    /// Analyze a frame and detect scene + state information
    pub fn analyze_frame(&mut self, image: &DynamicImage) -> Result<(Scene, State), AppError> {
        let analysis_start = Instant::now();

        // Create detection context
        let context = DetectionContext::new(image.clone());

        // Run detection pipeline to gather visual signals
        let pipeline_result = self.detection_pipeline.process(context);
        let enriched_context = pipeline_result.result;

        debug!(
            "Detection pipeline completed in {}us: {}",
            pipeline_result.processing_time_us, pipeline_result.reasoning
        );

        // Use scene detectors to determine the most likely scene
        let scene = self.detect_best_scene(&enriched_context)?;

        // Analyze game state for the detected scene
        let state_result = self.state_analyzer.analyze_state(&enriched_context, scene);
        let state = state_result.result;

        let total_time = analysis_start.elapsed().as_micros() as u64;
        info!(
            "Scene analysis completed in {}us: {:?} scene with {:?} location",
            total_time, scene, state.location_type
        );

        Ok((scene, state))
    }

    /// Synchronous scene detection for backward compatibility
    pub fn detect_scene_sync(&self, image: &DynamicImage) -> Scene {
        let context = DetectionContext::new(image.clone());

        // Quick scene detection without full pipeline
        self.detect_best_scene(&context).unwrap_or(Scene::Unknown)
    }

    /// Find the scene with highest confidence from all scene detectors
    fn detect_best_scene(&self, context: &DetectionContext) -> Result<Scene, AppError> {
        let mut best_scene = Scene::Unknown;
        let mut best_confidence = 0.0;

        for detector in &self.scene_detectors {
            let result = detector.detect_scene(context);

            debug!(
                "{}: {:?} with confidence {:.2}",
                detector.name(),
                result.result,
                result.confidence
            );

            if result.confidence > best_confidence {
                best_scene = result.result;
                best_confidence = result.confidence;
            }
        }

        // Use confidence threshold from config
        if best_confidence >= self.config.confidence_threshold {
            Ok(best_scene)
        } else {
            // Fallback to Unknown if no detector is confident enough
            Ok(Scene::Unknown)
        }
    }

    /// Get configuration for debugging
    pub fn get_config(&self) -> &SceneAnalysisConfig {
        &self.config
    }

    /// Get pipeline statistics
    pub fn get_pipeline_stats(&self) -> super::pipeline::PipelineStats {
        self.detection_pipeline.get_stats()
    }

    /// Update configuration at runtime
    pub fn update_config(&mut self, new_config: SceneAnalysisConfig) -> Result<(), AppError> {
        new_config
            .validate()
            .map_err(|e| AppError::Client(format!("Invalid config: {}", e)))?;
        self.config = new_config;
        Ok(())
    }
}

impl Clone for SceneAnalysisOrchestrator {
    fn clone(&self) -> Self {
        // Note: This creates a new instance with the same configuration
        // The actual detectors are recreated to avoid shared mutable state
        Self::new(self.config.clone()).expect("Config should be valid")
    }
}

/// Tower Service implementation for backward compatibility
impl Service<EnrichedFrame> for SceneAnalysisOrchestrator {
    type Response = EnrichedFrame;
    type Error = AppError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, mut enriched_frame: EnrichedFrame) -> Self::Future {
        let (scene, state) = match self.analyze_frame(&enriched_frame.image) {
            Ok((s, st)) => (s, st),
            Err(e) => {
                tracing::error!("Scene analysis failed: {}", e);
                // Fallback to simple detection
                let scene = self.detect_scene_sync(&enriched_frame.image);
                let state = State {
                    scene,
                    player_position: (0.0, 0.0),
                    pokemon_count: 0,
                    current_location: None,
                    location_type: LocationType::Unknown,
                    pokemon_party: Vec::new(),
                    pokedex_seen: 0,
                    pokedex_caught: 0,
                    badges_earned: 0,
                    story_progress: StoryProgress::GameStart,
                    in_tall_grass: false,
                    menu_cursor_position: None,
                    battle_turn: None,
                    last_encounter_steps: 0,
                    encounter_chain: 0,
                };
                (scene, state)
            }
        };

        // Update the enriched frame with detected information
        if let Some(existing_state) = &mut enriched_frame.state {
            // Update existing state with new detection
            existing_state.scene = scene;
            existing_state.location_type = state.location_type;
            existing_state.in_tall_grass = state.in_tall_grass;
            // Keep existing counts and progress if available
        } else {
            enriched_frame.state = Some(state);
        }

        info!("Scene detected: {:?}", scene);

        Box::pin(async move { Ok(enriched_frame) })
    }
}

/// Factory for creating different scene analysis configurations
pub struct SceneAnalysisFactory;

impl SceneAnalysisFactory {
    /// Create orchestrator with default configuration
    pub fn create_default() -> Result<SceneAnalysisOrchestrator, AppError> {
        SceneAnalysisOrchestrator::new(SceneAnalysisConfig::default())
    }

    /// Create orchestrator optimized for speed
    pub fn create_speed_optimized() -> Result<SceneAnalysisOrchestrator, AppError> {
        SceneAnalysisOrchestrator::new(SceneAnalysisConfig::speed_optimized())
    }

    /// Create orchestrator optimized for accuracy
    pub fn create_accuracy_optimized() -> Result<SceneAnalysisOrchestrator, AppError> {
        SceneAnalysisOrchestrator::new(SceneAnalysisConfig::accuracy_optimized())
    }

    /// Create orchestrator optimized for Pokemon games
    pub fn create_pokemon_optimized() -> Result<SceneAnalysisOrchestrator, AppError> {
        SceneAnalysisOrchestrator::new(SceneAnalysisConfig::pokemon_optimized())
    }

    /// Create orchestrator with custom configuration
    pub fn create_custom(
        config: SceneAnalysisConfig,
    ) -> Result<SceneAnalysisOrchestrator, AppError> {
        SceneAnalysisOrchestrator::new(config)
    }
}
