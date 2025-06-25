use std::pin::Pin;

use image::DynamicImage;
use tokio::sync::broadcast;

use crate::error::FrameError;
use crate::intake::frame::Frame;
use crate::intake::frame::reader::FrameReader;

pub struct EmulatorReader {
    frame_rx: broadcast::Receiver<DynamicImage>,
}

impl EmulatorReader {
    pub fn new(frame_rx: broadcast::Receiver<DynamicImage>) -> Self {
        Self { frame_rx }
    }
}
impl FrameReader for EmulatorReader {
    fn read<'a>(
        &'a mut self,
    ) -> Pin<Box<dyn Future<Output = Result<Frame, FrameError>> + Send + 'a>> {
        Box::pin(async move {
            loop {
                match self.frame_rx.recv().await {
                    Ok(frame) => {
                        let frame = Frame::Image { image: frame };
                        return Ok(frame);
                    }
                    Err(e) => return Err(FrameError::Send(e.to_string())),
                }
            }
        })
    }
}
