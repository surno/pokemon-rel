use super::configuration::{ActionSelectionStrategy, PipelineConfiguration};
use crate::error::AppError;
use crate::pipeline::services::{
    image::scene_annotation_service::SceneAnnotationService,
    learning::{
        experience_collector::ExperienceCollector,
        reward::{
            calculator::navigation_reward::NavigationRewardCalculator,
            processor::{
                multi_objective_reward_processor::MultiObjectiveRewardProcessor,
                reward_processor::RewardProcessor,
            },
        },
        smart_action_service::SmartActionService,
    },
    managers::{ClientStateManager, ImageChangeDetector, MacroManager},
    orchestration::{
        AIPipelineOrchestrator, MetricsCollector, ProcessingPipeline,
        action_selector::{
            HybridActionSelector, PolicyBasedActionSelector, RuleBasedActionSelector,
        },
        metrics::{DebugTracker, PerformanceMonitor},
    },
    steps::{
        ActionSelectionStep, ImageChangeDetectionStep, LearningStep, MacroExecutionStep,
        PolicyInferenceStep, SceneAnalysisStep,
    },
};
use crate::pipeline::{GameAction, RLService};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use uuid::Uuid;

/// Factory for creating configured AI pipeline instances
/// This implements the Factory pattern to encapsulate the complex creation logic
pub struct AIPipelineFactory;

impl AIPipelineFactory {
    /// Create a fully configured AI pipeline orchestrator
    pub fn create_pipeline(
        config: PipelineConfiguration,
        action_tx: mpsc::Sender<(Uuid, GameAction)>,
    ) -> Result<AIPipelineOrchestrator, AppError> {
        // Validate configuration
        config
            .validate()
            .map_err(|e| AppError::Client(format!("Invalid configuration: {}", e)))?;

        // Create core services
        let scene_annotation_service = SceneAnnotationService::new(());
        let smart_action_service = Arc::new(Mutex::new(SmartActionService::new()));
        let rl_service = RLService::new();

        // Create managers
        let macro_manager = MacroManager::new();
        let image_change_detector = ImageChangeDetector::new()
            .with_threshold(config.image_change_threshold)
            .with_history_window(config.image_change_history_window);
        let client_state_manager =
            ClientStateManager::new().with_max_history(config.max_client_history);

        // Create learning components
        let reward_processor: Arc<Mutex<dyn RewardProcessor>> = Arc::new(Mutex::new(
            MultiObjectiveRewardProcessor::new(Box::new(NavigationRewardCalculator::default())),
        ));

        let (training_tx, _training_rx) = mpsc::channel(1000);
        let experience_collector = Arc::new(tokio::sync::Mutex::new(ExperienceCollector::new(
            10_000,
            training_tx,
        )));

        // Create action selector based on strategy
        let action_selector = Self::create_action_selector(&config.action_selection_strategy)?;

        // Create metrics collector with observers
        let mut metrics_collector = MetricsCollector::new();

        if config.performance_monitoring_enabled {
            metrics_collector = metrics_collector.add_observer(Box::new(PerformanceMonitor::new()));
        }

        if config.debug_tracking_enabled {
            metrics_collector = metrics_collector.add_observer(Box::new(DebugTracker::new()));
        }

        // Create processing pipeline with all steps
        let pipeline = Self::create_processing_pipeline(
            scene_annotation_service,
            smart_action_service,
            rl_service,
            action_selector,
            macro_manager,
            image_change_detector,
            client_state_manager,
            reward_processor,
            experience_collector,
            config.policy_update_frequency,
        )?;

        // Create and return the orchestrator
        Ok(AIPipelineOrchestrator::new(
            pipeline,
            action_tx,
            metrics_collector,
        ))
    }

    /// Create action selector based on strategy
    fn create_action_selector(
        strategy: &ActionSelectionStrategy,
    ) -> Result<Box<dyn crate::pipeline::services::orchestration::ActionSelector>, AppError> {
        match strategy {
            ActionSelectionStrategy::PolicyBased => Ok(Box::new(PolicyBasedActionSelector)),
            ActionSelectionStrategy::RuleBased => Ok(Box::new(RuleBasedActionSelector)),
            ActionSelectionStrategy::Hybrid { policy_weight } => {
                Ok(Box::new(HybridActionSelector::new(*policy_weight)))
            }
        }
    }

    /// Create the complete processing pipeline
    fn create_processing_pipeline(
        scene_annotation_service: SceneAnnotationService,
        smart_action_service: Arc<Mutex<SmartActionService>>,
        rl_service: RLService,
        action_selector: Box<dyn crate::pipeline::services::orchestration::ActionSelector>,
        macro_manager: MacroManager,
        image_change_detector: ImageChangeDetector,
        client_state_manager: ClientStateManager,
        reward_processor: Arc<Mutex<dyn RewardProcessor>>,
        experience_collector: Arc<tokio::sync::Mutex<ExperienceCollector>>,
        policy_update_frequency: usize,
    ) -> Result<ProcessingPipeline, AppError> {
        // Create shared RL service for learning step
        let rl_service_for_learning = Arc::new(Mutex::new(rl_service));

        Ok(ProcessingPipeline::new()
            // Step 1: Scene analysis and situation understanding
            .add_step(Box::new(SceneAnalysisStep::new(
                scene_annotation_service,
                smart_action_service,
            )))
            // Step 2: Policy inference
            .add_step(Box::new(PolicyInferenceStep::new(
                RLService::new(), // Create a new instance for this step
            )))
            // Step 3: Image change detection and client state management
            .add_step(Box::new(ImageChangeDetectionStep::new(
                image_change_detector,
                client_state_manager,
            )))
            // Step 4: Action selection using configured strategy
            .add_step(Box::new(ActionSelectionStep::new(action_selector)))
            // Step 5: Macro execution and management
            .add_step(Box::new(MacroExecutionStep::new(macro_manager)))
            // Step 6: Learning (reward processing, experience collection, policy updates)
            .add_step(Box::new(
                LearningStep::new(
                    reward_processor,
                    experience_collector,
                    rl_service_for_learning,
                )
                .with_policy_update_frequency(policy_update_frequency),
            )))
    }

    /// Create pipeline with default configuration
    pub fn create_default_pipeline(
        action_tx: mpsc::Sender<(Uuid, GameAction)>,
    ) -> Result<AIPipelineOrchestrator, AppError> {
        Self::create_pipeline(PipelineConfiguration::default(), action_tx)
    }

    /// Create pipeline optimized for performance
    pub fn create_performance_pipeline(
        action_tx: mpsc::Sender<(Uuid, GameAction)>,
    ) -> Result<AIPipelineOrchestrator, AppError> {
        Self::create_pipeline(PipelineConfiguration::performance_optimized(), action_tx)
    }

    /// Create pipeline optimized for learning
    pub fn create_learning_pipeline(
        action_tx: mpsc::Sender<(Uuid, GameAction)>,
    ) -> Result<AIPipelineOrchestrator, AppError> {
        Self::create_pipeline(PipelineConfiguration::learning_optimized(), action_tx)
    }

    /// Create pipeline optimized for debugging
    pub fn create_debug_pipeline(
        action_tx: mpsc::Sender<(Uuid, GameAction)>,
    ) -> Result<AIPipelineOrchestrator, AppError> {
        Self::create_pipeline(PipelineConfiguration::debug_optimized(), action_tx)
    }
}
