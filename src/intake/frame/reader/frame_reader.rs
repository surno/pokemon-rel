use crate::{error::FrameError, intake::frame::Frame};
use std::future::Future;
use std::pin::Pin;

pub enum ReadState {
    WaitingForLength,
    WaitingForFrame { expected_length: u32 },
}

pub trait FrameReader: Send + Sync {
    fn read<'a>(
        &'a mut self,
    ) -> Pin<Box<dyn Future<Output = Result<Frame, FrameError>> + Send + 'a>>;
}
