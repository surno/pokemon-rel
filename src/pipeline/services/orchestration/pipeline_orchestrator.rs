use super::{FrameContext, MetricsCollector, ProcessingPipeline, UIPipelineAdapter};
use crate::error::AppError;
use crate::pipeline::{EnrichedFrame, GameAction};
use std::time::Instant;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Main orchestrator for the AI pipeline - much simpler and focused!
/// This replaces the monolithic AIPipelineService with a clean, single-responsibility design
pub struct AIPipelineOrchestrator {
    pipeline: ProcessingPipeline,
    action_transmitter: ActionTransmitter,
    metrics_collector: Arc<tokio::sync::Mutex<MetricsCollector>>,
    ui_adapter: UIPipelineAdapter,
}

impl AIPipelineOrchestrator {
    pub fn new(
        pipeline: ProcessingPipeline,
        action_tx: mpsc::Sender<(Uuid, GameAction)>,
        metrics_collector: MetricsCollector,
        ui_adapter: UIPipelineAdapter,
    ) -> Self {
        Self {
            pipeline,
            action_transmitter: ActionTransmitter::new(action_tx),
            metrics_collector: Arc::new(tokio::sync::Mutex::new(metrics_collector)),
            ui_adapter,
        }
    }

    /// Process a single frame through the pipeline
    pub async fn process_frame(&mut self, frame: EnrichedFrame) -> Result<(), AppError> {
        let client_id = frame.client;
        let frame_start = Instant::now();

        debug!("Processing frame for client {}", client_id);

        // Create processing context
        let context = FrameContext::new(frame);

        // Process through the pipeline
        let mut processed_context = self.pipeline.process(context).await?;

        // Update UI adapter with decision history if available
        if let Some(smart_decision) = &processed_context.smart_decision {
            self.ui_adapter
                .add_client_decision(client_id, smart_decision.clone());
        }

        // Finalize metrics
        processed_context.metrics.finalize(frame_start);

        // Send action if one was selected
        if let Some(action) = processed_context.selected_action {
            self.action_transmitter
                .send_action(client_id, action)
                .await?;

            // Notify metrics observers
            let mut collector = self.metrics_collector.lock().await;
            collector.notify_action_sent(client_id, action);
        }

        // Notify metrics observers about frame completion and individual step timings
        let mut collector = self.metrics_collector.lock().await;
        collector.notify_frame_processed(client_id, &processed_context.metrics);

        // Notify individual processing step timings for bottleneck detection
        use crate::pipeline::services::orchestration::frame_context::ProcessingStepType;
        collector.notify_processing_step(
            client_id,
            ProcessingStepType::SceneAnalysis,
            processed_context.metrics.scene_analysis_duration_us,
        );
        collector.notify_processing_step(
            client_id,
            ProcessingStepType::PolicyInference,
            processed_context.metrics.policy_inference_duration_us,
        );
        collector.notify_processing_step(
            client_id,
            ProcessingStepType::ActionSelection,
            processed_context.metrics.action_selection_duration_us,
        );
        collector.notify_processing_step(
            client_id,
            ProcessingStepType::MacroExecution,
            processed_context.metrics.macro_execution_duration_us,
        );
        collector.notify_processing_step(
            client_id,
            ProcessingStepType::RewardProcessing,
            processed_context.metrics.reward_processing_duration_us,
        );
        collector.notify_processing_step(
            client_id,
            ProcessingStepType::ExperienceCollection,
            processed_context.metrics.experience_collection_duration_us,
        );
        collector.notify_processing_step(
            client_id,
            ProcessingStepType::ImageChangeDetection,
            processed_context.metrics.image_change_detection_us,
        );

        info!(
            "Frame processed for client {} in {}us",
            client_id, processed_context.metrics.total_processing_duration_us
        );

        Ok(())
    }

    /// Start processing frames from a receiver channel
    pub async fn start_processing(
        mut self,
        mut frame_rx: mpsc::Receiver<EnrichedFrame>,
    ) -> Result<(), AppError> {
        info!("AI Pipeline Orchestrator started - waiting for frames...");

        while let Some(frame) = frame_rx.recv().await {
            if let Err(e) = self.process_frame(frame).await {
                error!("Error processing frame: {}", e);
            }
        }

        info!("AI Pipeline Orchestrator stopped");
        Ok(())
    }

    /// Synchronous frame processing for use in GUI contexts
    pub fn process_frame_sync(&mut self, frame: EnrichedFrame) -> Result<(), AppError> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| AppError::Client(format!("Failed to create runtime: {}", e)))?;

        rt.block_on(self.process_frame(frame))
    }

    pub async fn get_metrics_collector(&self) -> tokio::sync::MutexGuard<'_, MetricsCollector> {
        self.metrics_collector.lock().await
    }

    /// Get UI adapter for backward compatibility with existing UI code
    pub fn get_ui_adapter(&self) -> UIPipelineAdapter {
        self.ui_adapter.clone()
    }
}

/// Handles sending actions to clients
pub struct ActionTransmitter {
    action_tx: mpsc::Sender<(Uuid, GameAction)>,
}

impl ActionTransmitter {
    pub fn new(action_tx: mpsc::Sender<(Uuid, GameAction)>) -> Self {
        Self { action_tx }
    }

    pub async fn send_action(&self, client_id: Uuid, action: GameAction) -> Result<(), AppError> {
        if let Err(e) = self.action_tx.try_send((client_id, action)) {
            warn!("Failed to send action to client {}: {}", client_id, e);
            return Err(AppError::Client(format!("Failed to send action: {}", e)));
        }
        debug!("Sent action {:?} to client {}", action, client_id);
        Ok(())
    }
}
