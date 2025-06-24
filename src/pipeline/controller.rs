use crate::{error::AppError, pipeline::EnrichedFrame};
use tokio::sync::broadcast;

pub struct AppController {
    frame_rx: broadcast::Receiver<EnrichedFrame>,
    result_tx: broadcast::Sender<EnrichedFrame>,
}

impl AppController {
    pub fn new(
        frame_rx: broadcast::Receiver<EnrichedFrame>,
        result_tx: broadcast::Sender<EnrichedFrame>,
    ) -> Self {
        Self {
            frame_rx,
            result_tx,
        }
    }

    pub async fn run(&self) -> Result<(), AppError> {
        loop {
            tokio::select! {
                Ok(frame) = self.frame_rx.recv() => {
                    // self.result_tx.send(frame).map_err(|e| AppError::Pipeline(e.to_string()))?;
                }
            }
        }
    }
}
