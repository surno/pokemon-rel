use std::future::Future;
use std::pin::Pin;

use tokio::sync::mpsc;

use crate::error::FrameError;
use crate::intake::frame::writer::FramedWriter;
use crate::pipeline::GameAction;

pub struct EmulatorWriter {
    frame_tx: mpsc::Sender<GameAction>,
}

impl EmulatorWriter {
    pub fn new(frame_tx: mpsc::Sender<GameAction>) -> Self {
        Self { frame_tx }
    }
}

impl FramedWriter for EmulatorWriter {
    fn write(
        &mut self,
        action: GameAction,
    ) -> Pin<Box<dyn Future<Output = Result<(), FrameError>> + Send>> {
        Box::pin(async move {
            self.frame_tx
                .send(action)
                .await
                .map_err(|e| FrameError::Send(e.to_string()))?;
            Ok(())
        })
    }
}
