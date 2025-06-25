use tokio::sync::mpsc;

use crate::error::FrameError;
use crate::intake::frame::Frame;
use crate::intake::frame::writer::FrameWriter;

pub struct EmulatorWriter {
    frame_tx: mpsc::Sender<Frame>,
}

impl EmulatorWriter {
    pub fn new(frame_tx: mpsc::Sender<Frame>) -> Self {
        Self { frame_tx }
    }
}

impl FrameWriter for EmulatorWriter {
    fn write(&mut self, _: Frame) -> Result<(), FrameError> {
        Ok(())
    }
}
