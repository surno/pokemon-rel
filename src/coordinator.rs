use crate::pipeline::context::frame_context::FrameContext;
use crate::pipeline::context::state::AnalyzedState;
use crate::{
    common::{frame::Frame, game_action::GameAction},
    config::Configuration,
    emulator::emulator_client::{self, EmulatorClient},
    error::AppError,
    pipeline::orchestration::processing_pipeline::ProcessingPipeline,
};
use tokio::sync::watch::Receiver;
use tokio_util::sync::CancellationToken;

pub struct Coordinator {
    pipeline_task: tokio::task::JoinHandle<()>,
    cancel_token: CancellationToken,
    frame_rx: Option<Receiver<Option<FrameContext<AnalyzedState>>>>,
}

impl Coordinator {
    fn new(configuration: Configuration, pipeline: ProcessingPipeline, desmume: Option<desmume_rs::DeSmuME>) -> Self {
        let cancel_token = CancellationToken::new();

        let (pipeline_task, frame_rx) =
            Self::start_tasks(configuration, pipeline, cancel_token.clone(), desmume);
        Self {
            pipeline_task,
            cancel_token,
            frame_rx: Some(frame_rx),
        }
    }

    pub fn frame_rx(&mut self) -> Option<Receiver<Option<FrameContext<AnalyzedState>>>> {
        // move the frame_rx out of the coordinator
        self.frame_rx.take()
    }

    fn start_tasks(
        configuration: Configuration,
        pipeline: ProcessingPipeline,
        cancel_token: CancellationToken,
        desmume: Option<desmume_rs::DeSmuME>,
    ) -> (
        tokio::task::JoinHandle<()>,
        Receiver<Option<FrameContext<AnalyzedState>>>,
    ) {
        let (frame_tx, frame_rx) = tokio::sync::mpsc::channel(configuration.frame_buffer_size);
        let (action_tx, action_rx) = tokio::sync::mpsc::channel(configuration.action_buffer_size);
        
        // Use pre-initialized emulator if provided, otherwise initialize here (fallback for non-Metal cases)
        let desmume = match desmume {
            Some(emu) => {
                tracing::info!("Using pre-initialized emulator (initialized on main thread)");
                emu
            }
            None => {
                tracing::info!("Initializing emulator in start_tasks (fallback mode)");
                match emulator_client::initialize_emulator(
                    configuration.rom_path.clone(),
                    configuration.renderer.clone(),
                ) {
                    Ok(emu) => emu,
                    Err(e) => {
                        tracing::error!("Failed to initialize emulator: {}", e);
                        // Return a dummy task handle - the actual error should be handled by caller
                        let (dummy_task, dummy_rx) = Self::start_pipeline_task(pipeline, frame_rx, cancel_token.clone());
                        return (dummy_task, dummy_rx);
                    }
                }
            }
        };
        
        let mut client = EmulatorClient::new(
            action_rx,
            frame_tx,
            desmume,
        );
        let (pipeline_task, pipeline_frame_rx) =
            Self::start_pipeline_task(pipeline, frame_rx, cancel_token.clone());
        let handler_task = tokio::spawn(async move {
            tokio::select! {
                    _ = cancel_token.cancelled() => {
                        client.stop();
                        pipeline_task.await.unwrap();
                        action_tx.send(GameAction::A).await.unwrap();
                    }
            }
        });
        (handler_task, pipeline_frame_rx)
    }

    fn start_pipeline_task(
        mut pipeline: ProcessingPipeline,
        mut frame_rx: tokio::sync::mpsc::Receiver<Frame>,
        cancel_token: CancellationToken,
    ) -> (
        tokio::task::JoinHandle<()>,
        Receiver<Option<FrameContext<AnalyzedState>>>,
    ) {
        let (pipeline_frame_tx, pipeline_frame_rx) = tokio::sync::watch::channel(None);
        let pipeline_task = tokio::spawn(async move {
            while let Some(frame) = frame_rx.recv().await
                && !cancel_token.is_cancelled()
            {
                let response = pipeline.process(frame).await;
                match response {
                    Ok(frame_context) => {
                        if let Err(e) = pipeline_frame_tx.send(Some(frame_context)) {
                            tracing::error!("Failed to send frame context to pipeline: {}", e);
                        }
                    }
                    Err(e) => {
                        tracing::error!("Pipeline error: {}", e);
                        break;
                    }
                }
            }
        });
        (pipeline_task, pipeline_frame_rx)
    }

    pub fn stop(&self) {
        self.cancel_token.cancel();
        self.pipeline_task.abort();
    }
}

impl Drop for Coordinator {
    fn drop(&mut self) {
        self.stop();
    }
}

pub struct CoordinatorBuilder {
    configuration: Configuration,
    pipeline: Option<ProcessingPipeline>,
    desmume: Option<desmume_rs::DeSmuME>,
}

impl CoordinatorBuilder {
    pub fn new(configuration: Configuration) -> Self {
        Self {
            configuration,
            pipeline: None,
            desmume: None,
        }
    }

    // Sets the ROM path, this will override the default configuration.
    pub fn rom_path(mut self, rom_path: String) -> Self {
        self.configuration.rom_path = rom_path;
        self
    }

    // Adjusts the frame buffer size, this will override the default configuration.
    pub fn frame_buffer_size(mut self, frame_buffer_size: usize) -> Self {
        self.configuration.frame_buffer_size = frame_buffer_size;
        self
    }

    // Sets the action buffer size, this will override the default configuration.
    pub fn action_buffer_size(mut self, action_buffer_size: usize) -> Self {
        self.configuration.action_buffer_size = action_buffer_size;
        self
    }

    // Enables metrics, this will override the default configuration.
    pub fn enable_metrics(mut self, enable_metrics: bool) -> Self {
        self.configuration.enable_metrics = enable_metrics;
        self
    }

    pub fn pipeline(mut self, pipeline: ProcessingPipeline) -> Self {
        self.pipeline = Some(pipeline);
        self
    }

    // Sets a pre-initialized DeSmuME instance (required for Metal thread safety on macOS).
    // The emulator must be initialized on the main thread before the tokio runtime is created.
    pub fn desmume(mut self, desmume: desmume_rs::DeSmuME) -> Self {
        self.desmume = Some(desmume);
        self
    }

    pub fn build(self) -> Result<Coordinator, AppError> {
        let pipeline = self
            .pipeline
            .ok_or(AppError::Pipeline("Pipeline not set".to_string()))?;
        Ok(Coordinator::new(self.configuration, pipeline, self.desmume))
    }
}

#[cfg(test)]
mod tests {
    use crate::pipeline::orchestration::step::scene_analyzer::SceneAnalyzer;

    use super::*;

    #[tokio::test]
    async fn test_coordinator() {
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
        coordinator.stop();
    }
}
