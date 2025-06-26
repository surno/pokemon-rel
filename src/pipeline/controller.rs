use crate::pipeline::services::image::SceneAnnotationService;
use crate::{
    error::AppError, intake::client::supervisor::ClientSupervisorCommand, pipeline::EnrichedFrame,
};
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tower::Service;
use tracing::{debug, error};

pub struct AppController {
    frame_rx: mpsc::Receiver<EnrichedFrame>,
    result_tx: mpsc::Sender<EnrichedFrame>,
    action_tx: mpsc::Sender<ClientSupervisorCommand>,
    scene_annotation_service: SceneAnnotationService,
}

impl AppController {
    pub fn new(
        frame_rx: mpsc::Receiver<EnrichedFrame>,
        result_tx: mpsc::Sender<EnrichedFrame>,
        action_tx: mpsc::Sender<ClientSupervisorCommand>,
        scene_annotation_service: SceneAnnotationService,
    ) -> Self {
        Self {
            frame_rx,
            result_tx,
            action_tx,
            scene_annotation_service,
        }
    }

    pub async fn run(&mut self) -> Result<(), AppError> {
        loop {
            tokio::select! {
                Some(frame) = self.frame_rx.recv() => {
                    let scene = self.scene_annotation_service.call(frame).await.unwrap();
                    self.result_tx.send(scene).await.unwrap();
                }
                else => {
                    debug!("Frame receiver task died");
                    return Err(AppError::Pipeline("Frame receiver task died".to_string()));
                }
            }
        }
    }
}
