use crate::{error::AppError, intake::frame::writer::FramedWriter, pipeline::GameAction};
use std::{future::Future, pin::Pin};
use tokio::sync::mpsc;

pub struct EmulatorWriter {
    frame_tx: mpsc::Sender<GameAction>,
}

impl EmulatorWriter {
    pub fn new(frame_tx: mpsc::Sender<GameAction>) -> Self {
        Self { frame_tx }
    }
}

impl FramedWriter for EmulatorWriter {
    fn send_action<'a>(
        &'a mut self,
        action: GameAction,
    ) -> Pin<Box<dyn Future<Output = Result<(), AppError>> + Send + 'a>> {
        Box::pin(async move {
            self.frame_tx
                .send(action)
                .await
                .map_err(|e| AppError::Client(e.to_string()))
        })
    }
}
