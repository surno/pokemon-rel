use std::pin::Pin;

use image::DynamicImage;
use tokio::sync::mpsc;

use crate::error::FrameError;
use crate::intake::frame::Frame;
use crate::intake::frame::reader::FrameReader;

pub struct EmulatorReader {
    frame_rx: mpsc::Receiver<DynamicImage>,
}

impl EmulatorReader {
    pub fn new(frame_rx: mpsc::Receiver<DynamicImage>) -> Self {
        Self { frame_rx }
    }
}
impl FrameReader for EmulatorReader {
    fn read<'a>(
        &'a mut self,
    ) -> Pin<Box<dyn Future<Output = Result<Frame, FrameError>> + Send + 'a>> {
        Box::pin(async move {
            match self.frame_rx.recv().await {
                Some(frame) => {
                    let frame = Frame::Image { image: frame };
                    return Ok(frame);
                }
                None => return Err(FrameError::Send("Channel closed".to_string())),
            }
        })
    }
}
