use crate::{
    error::AppError, intake::client::supervisor::ClientSupervisorCommand, pipeline::EnrichedFrame,
};
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tracing::{debug, error};

pub struct AppController {
    frame_rx: mpsc::Receiver<EnrichedFrame>,
    result_tx: broadcast::Sender<EnrichedFrame>,
    action_tx: mpsc::Sender<ClientSupervisorCommand>,
}

impl AppController {
    pub fn new(
        frame_rx: mpsc::Receiver<EnrichedFrame>,
        result_tx: broadcast::Sender<EnrichedFrame>,
        action_tx: mpsc::Sender<ClientSupervisorCommand>,
    ) -> Self {
        Self {
            frame_rx,
            result_tx,
            action_tx,
        }
    }

    pub async fn run(&mut self) -> Result<(), AppError> {
        loop {
            debug!("Waiting for frame from client");
            tokio::select! {
                Some(frame) = self.frame_rx.recv() => {
                    debug!("Received frame from client: {:?}, sending in controller.", frame.client);
                    match self.result_tx.send(frame) {
                        Ok(_) => {
                            debug!("Frame sent to result channel on gui");
                        }
                        Err(e) => {
                            error!("Error sending frame to result channel: {:?}", e.to_string());
                        }
                    }
                }
                else => {
                    debug!("Frame receiver task died");
                    return Err(AppError::Pipeline("Frame receiver task died".to_string()));
                }
            }
        }
    }
}
