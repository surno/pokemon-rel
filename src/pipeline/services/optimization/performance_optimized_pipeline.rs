/// Performance-optimized pipeline factory that creates ultra-fast pipelines

use crate::error::AppError;
use tower::Service;
use crate::pipeline::{GameAction, RLService};
use crate::pipeline::services::{
    factory::{AIPipelineFactory, PipelineConfiguration},
    factory::configuration::ActionSelectionStrategy,
    image::analysis::{SceneAnalysisOrchestrator, SceneAnalysisConfig},
    learning::{
        smart_action_service::SmartActionService,
        experience_collector::ExperienceCollector,
        reward::{
            calculator::navigation_reward::NavigationRewardCalculator,
            processor::{
                multi_objective_reward_processor::MultiObjectiveRewardProcessor,
                reward_processor::RewardProcessor,
            },
        },
    },
    managers::{ClientStateManager, MacroManager},
    orchestration::{
        AIPipelineOrchestrator, MetricsCollector, ProcessingPipeline, UIPipelineAdapter,
        action_selector::PolicyBasedActionSelector,
        metrics::{PerformanceMonitor, DebugTracker},
    },
    steps::{
        ActionSelectionStep, ImageChangeDetectionStep, LearningStep, MacroExecutionStep,
        PolicyInferenceStep, SceneAnalysisStep,
    },
    optimization::{FastSituationAnalyzer, FastImageChangeDetector},
};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use tokio::sync::mpsc;
use uuid::Uuid;

/// Factory for creating performance-optimized pipelines
pub struct PerformanceOptimizedPipelineFactory;

impl PerformanceOptimizedPipelineFactory {
    /// Create ultra-fast pipeline optimized for maximum FPS
    pub fn create_ultra_fast_pipeline(
        action_tx: mpsc::Sender<(Uuid, GameAction)>,
    ) -> Result<AIPipelineOrchestrator, AppError> {
        let config = PipelineConfiguration {
            action_selection_strategy: ActionSelectionStrategy::PolicyBased, // Fastest strategy
            image_change_threshold: 8, // Less sensitive = faster
            image_change_history_window: 1, // Minimal history
            max_client_history: 20, // Less memory usage
            policy_update_frequency: 200, // Less frequent updates
            performance_monitoring_enabled: true,
            debug_tracking_enabled: false, // Disable for max performance
        };

        Self::create_optimized_pipeline(config, action_tx, OptimizationLevel::UltraFast)
    }

    /// Create balanced pipeline with good performance and reasonable accuracy
    pub fn create_fast_pipeline(
        action_tx: mpsc::Sender<(Uuid, GameAction)>,
    ) -> Result<AIPipelineOrchestrator, AppError> {
        let config = PipelineConfiguration {
            action_selection_strategy: ActionSelectionStrategy::PolicyBased,
            image_change_threshold: 6,
            image_change_history_window: 3,
            max_client_history: 50,
            policy_update_frequency: 100,
            performance_monitoring_enabled: true,
            debug_tracking_enabled: true,
        };

        Self::create_optimized_pipeline(config, action_tx, OptimizationLevel::Fast)
    }

    /// Create performance-optimized pipeline with custom optimization level
    fn create_optimized_pipeline(
        config: PipelineConfiguration,
        action_tx: mpsc::Sender<(Uuid, GameAction)>,
        optimization_level: OptimizationLevel,
    ) -> Result<AIPipelineOrchestrator, AppError> {
        config.validate().map_err(|e| AppError::Client(format!("Invalid config: {}", e)))?;

        // Create optimized scene analysis
        let scene_config = match optimization_level {
            OptimizationLevel::UltraFast => SceneAnalysisConfig::speed_optimized(),
            OptimizationLevel::Fast => SceneAnalysisConfig::pokemon_optimized(),
        };
        
        let scene_analysis_orchestrator = SceneAnalysisOrchestrator::new(scene_config)
            .map_err(|e| AppError::Client(format!("Failed to create scene analysis: {}", e)))?;

        // Create optimized smart action service with fast situation analyzer
        let smart_action_service = Arc::new(Mutex::new(SmartActionService::new()));

        let rl_service = RLService::new();

        // Create optimized managers
        let macro_manager = MacroManager::new();
        
        // Use fast image change detector instead of slow perceptual hashing
        let fast_image_change_detector = FastImageChangeDetector::new()
            .with_threshold(0.05) // 5% change threshold
            .with_sample_rate(match optimization_level {
                OptimizationLevel::UltraFast => 32, // Sample every 32nd pixel
                OptimizationLevel::Fast => 16,      // Sample every 16th pixel
            });

        let client_state_manager = ClientStateManager::new()
            .with_max_history(config.max_client_history);

        // Create lightweight learning components
        let reward_processor: Arc<Mutex<dyn RewardProcessor>> = Arc::new(Mutex::new(
            MultiObjectiveRewardProcessor::new(Box::new(NavigationRewardCalculator::default()))
        ));
        
        let (training_tx, _training_rx) = mpsc::channel(1000);
        let experience_collector = Arc::new(tokio::sync::Mutex::new(
            ExperienceCollector::new(5_000, training_tx) // Smaller buffer for speed
        ));

        // Create action selector
        let action_selector = Box::new(PolicyBasedActionSelector);

        // Create optimized metrics collector
        let performance_monitor = PerformanceMonitor::new();
        let performance_stats = performance_monitor.get_stats_shared();

        let mut metrics_collector = MetricsCollector::new()
            .add_observer(Box::new(performance_monitor));

        if config.debug_tracking_enabled {
            let debug_tracker = DebugTracker::new();
            let debug_info = debug_tracker.get_debug_info_shared();
            metrics_collector = metrics_collector.add_observer(Box::new(debug_tracker));

            let ui_adapter = UIPipelineAdapter::new(
                performance_stats,
                Arc::new(Mutex::new(HashMap::new())),
                debug_info,
            );

            // Create optimized processing pipeline
            let pipeline = Self::create_optimized_processing_pipeline(
                scene_analysis_orchestrator,
                smart_action_service,
                rl_service,
                action_selector,
                macro_manager,
                fast_image_change_detector,
                client_state_manager,
                reward_processor,
                experience_collector,
                config.policy_update_frequency,
                optimization_level,
            )?;

            Ok(AIPipelineOrchestrator::new(pipeline, action_tx, metrics_collector, ui_adapter))
        } else {
            let ui_adapter = UIPipelineAdapter::new(
                performance_stats,
                Arc::new(Mutex::new(HashMap::new())),
                Arc::new(Mutex::new(crate::pipeline::services::orchestration::metrics::DebugInfo::default())),
            );

            // Create optimized processing pipeline
            let pipeline = Self::create_optimized_processing_pipeline(
                scene_analysis_orchestrator,
                smart_action_service,
                rl_service,
                action_selector,
                macro_manager,
                fast_image_change_detector,
                client_state_manager,
                reward_processor,
                experience_collector,
                config.policy_update_frequency,
                optimization_level,
            )?;

            Ok(AIPipelineOrchestrator::new(pipeline, action_tx, metrics_collector, ui_adapter))
        }
    }

    /// Create optimized processing pipeline with performance tweaks
    fn create_optimized_processing_pipeline(
        scene_analysis_orchestrator: SceneAnalysisOrchestrator,
        smart_action_service: Arc<Mutex<SmartActionService>>,
        rl_service: RLService,
        action_selector: Box<dyn crate::pipeline::services::orchestration::ActionSelector>,
        macro_manager: MacroManager,
        fast_image_change_detector: FastImageChangeDetector,
        client_state_manager: ClientStateManager,
        reward_processor: Arc<Mutex<dyn RewardProcessor>>,
        experience_collector: Arc<tokio::sync::Mutex<ExperienceCollector>>,
        policy_update_frequency: usize,
        optimization_level: OptimizationLevel,
    ) -> Result<ProcessingPipeline, AppError> {
        let rl_service_for_learning = Arc::new(Mutex::new(rl_service));

        let mut pipeline = ProcessingPipeline::new()
            // Step 1: Fast scene analysis
            .add_step(Box::new(FastSceneAnalysisStep::new(
                scene_analysis_orchestrator,
                smart_action_service,
                optimization_level,
            )))
            // Step 2: Policy inference (already fast)
            .add_step(Box::new(PolicyInferenceStep::new(RLService::new())))
            // Step 3: Fast image change detection  
            .add_step(Box::new(FastImageChangeDetectionStep::new(
                fast_image_change_detector,
                client_state_manager,
            )))
            // Step 4: Action selection
            .add_step(Box::new(ActionSelectionStep::new(action_selector)))
            // Step 5: Macro execution
            .add_step(Box::new(MacroExecutionStep::new(macro_manager)));

        // Only add learning step if not in ultra-fast mode
        if optimization_level != OptimizationLevel::UltraFast {
            pipeline = pipeline.add_step(Box::new(LearningStep::new(
                reward_processor,
                experience_collector,
                rl_service_for_learning,
            ).with_policy_update_frequency(policy_update_frequency)));
        }

        Ok(pipeline)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum OptimizationLevel {
    UltraFast, // Maximum speed, minimal features
    Fast,      // Good speed with essential features
}

/// Fast scene analysis step that uses optimized situation analyzer
pub struct FastSceneAnalysisStep {
    scene_analysis_orchestrator: SceneAnalysisOrchestrator,
    fast_situation_analyzer: FastSituationAnalyzer,
    optimization_level: OptimizationLevel,
}

impl FastSceneAnalysisStep {
    pub fn new(
        scene_analysis_orchestrator: SceneAnalysisOrchestrator,
        _smart_action_service: Arc<Mutex<SmartActionService>>,
        optimization_level: OptimizationLevel,
    ) -> Self {
        let fast_situation_analyzer = FastSituationAnalyzer::new()
            .with_expensive_analysis(optimization_level != OptimizationLevel::UltraFast);

        Self {
            scene_analysis_orchestrator,
            fast_situation_analyzer,
            optimization_level,
        }
    }
}

#[async_trait::async_trait]
impl crate::pipeline::services::orchestration::ProcessingStep for FastSceneAnalysisStep {
    async fn process(&mut self, context: &mut crate::pipeline::services::orchestration::FrameContext) -> Result<(), AppError> {
        let step_start = std::time::Instant::now();

        // Use fast situation analysis instead of expensive image processing
        let situation = self.fast_situation_analyzer.analyze_situation_fast(&context.frame);

        // Only do full scene analysis if we need high accuracy
        if self.optimization_level != OptimizationLevel::UltraFast {
            // Update frame with scene analysis (but don't wait for it)
            if let Ok(annotated_frame) = self.scene_analysis_orchestrator.call(context.frame.clone()).await {
                context.frame = annotated_frame;
            }
        }

        // Create a mock smart decision for compatibility
        let smart_decision = crate::pipeline::services::learning::smart_action_service::ActionDecision {
            action: crate::pipeline::GameAction::A, // Default action
            confidence: 0.7,
            reasoning: "Fast analysis decision".to_string(),
            expected_outcome: "Quick action".to_string(),
        };

        // Update context
        context.situation = Some(situation);
        context.smart_decision = Some(smart_decision);

        // Record timing
        let duration = step_start.elapsed().as_micros() as u64;
        context.metrics.record_duration(
            crate::pipeline::services::orchestration::frame_context::ProcessingStepType::SceneAnalysis, 
            duration
        );

        Ok(())
    }

    fn name(&self) -> &'static str {
        "FastSceneAnalysisStep"
    }
}

/// Fast image change detection step
pub struct FastImageChangeDetectionStep {
    fast_image_change_detector: FastImageChangeDetector,
    client_state_manager: ClientStateManager,
}

impl FastImageChangeDetectionStep {
    pub fn new(
        fast_image_change_detector: FastImageChangeDetector,
        client_state_manager: ClientStateManager,
    ) -> Self {
        Self {
            fast_image_change_detector,
            client_state_manager,
        }
    }
}

#[async_trait::async_trait]
impl crate::pipeline::services::orchestration::ProcessingStep for FastImageChangeDetectionStep {
    async fn process(&mut self, context: &mut crate::pipeline::services::orchestration::FrameContext) -> Result<(), AppError> {
        let step_start = std::time::Instant::now();

        // Fast image change detection
        let image_changed = self.fast_image_change_detector.detect_change_fast(
            context.client_id,
            &context.frame.image,
        );
        context.image_changed = image_changed;

        // Minimal client state updates for speed
        if let Some(situation) = context.situation.as_ref() {
            // Only update intro tracking (lightweight)
            self.client_state_manager.update_intro_tracking(
                context.client_id,
                situation.scene,
            );
        }

        // Record timing
        let duration = step_start.elapsed().as_micros() as u64;
        context.metrics.record_duration(
            crate::pipeline::services::orchestration::frame_context::ProcessingStepType::ImageChangeDetection, 
            duration
        );

        Ok(())
    }

    fn name(&self) -> &'static str {
        "FastImageChangeDetectionStep"
    }
}
