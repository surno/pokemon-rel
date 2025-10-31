use crate::{
    common::{frame::Frame, game_action::GameAction},
    config::Configuration,
    emulator::emulator_client::EmulatorClient,
    error::AppError,
    pipeline::orchestration::processing_pipeline::ProcessingPipeline,
};
use tokio::sync::mpsc::Receiver;
use tokio_util::sync::CancellationToken;

struct Coordinator {
    pipeline_task: tokio::task::JoinHandle<()>,
    cancel_token: CancellationToken,
}

impl Coordinator {
    fn new(configuration: Configuration, pipeline: ProcessingPipeline) -> Self {
        let cancel_token = CancellationToken::new();

        Self {
            pipeline_task: Self::start_tasks(configuration, pipeline, cancel_token.clone()),
            cancel_token,
        }
    }

    fn start_tasks(
        configuration: Configuration,
        pipeline: ProcessingPipeline,
        cancel_token: CancellationToken,
    ) -> tokio::task::JoinHandle<()> {
        let (frame_tx, frame_rx) = tokio::sync::mpsc::channel(configuration.frame_buffer_size);
        let (_action_tx, action_rx) = tokio::sync::mpsc::channel(configuration.action_buffer_size);
        let mut client = EmulatorClient::new(action_rx, frame_tx, configuration.rom_path.clone());
        let pipeline_task = Self::start_pipeline_task(pipeline, frame_rx, cancel_token.clone());
        let handler_task = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = cancel_token.cancelled() => {
                        client.stop();
                        pipeline_task.await.unwrap();
                        break;
                    }
                }
            }
        });
        handler_task
    }

    fn start_pipeline_task(
        mut pipeline: ProcessingPipeline,
        mut frame_rx: Receiver<Frame>,
        cancel_token: CancellationToken,
    ) -> tokio::task::JoinHandle<()> {
        let pipeline_task = tokio::spawn(async move {
            while let Some(frame) = frame_rx.recv().await
                && !cancel_token.is_cancelled()
            {
                let response = pipeline.process(frame).await;
                if let Err(e) = response {
                    tracing::error!("Pipeline error: {}", e);
                } else {
                    tracing::info!("Pipeline got response.");
                }
            }
        });
        pipeline_task
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
}

impl CoordinatorBuilder {
    pub fn new(configuration: Configuration) -> Self {
        Self {
            configuration,
            pipeline: None,
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

    pub fn build(self) -> Result<Coordinator, AppError> {
        let pipeline = self
            .pipeline
            .ok_or(AppError::Pipeline("Pipeline not set".to_string()))?;
        Ok(Coordinator::new(self.configuration, pipeline))
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
