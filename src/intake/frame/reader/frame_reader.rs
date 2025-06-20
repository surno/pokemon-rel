use crate::{error::FrameError, intake::frame::Frame};
use std::future::Future;
use std::pin::Pin;

pub enum ReadState {
    WaitingForLength,
    WaitingForFrame { expected_length: u32 },
}

pub trait FrameReader: Send + Sync {
    fn is_connected<'a>(&'a self) -> Pin<Box<dyn Future<Output = bool> + Send + 'a>>;
    fn read<'a>(
        &'a mut self,
    ) -> Pin<Box<dyn Future<Output = Result<Frame, FrameError>> + Send + 'a>>;
}
