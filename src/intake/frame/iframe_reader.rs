use crate::{error::FrameError, intake::frame::Frame};
use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;

#[derive(Debug)]
pub enum ReadState {
    WaitingForLength,
    WaitingForFrame { expected_length: u32 },
}

pub trait IFrameReader: Debug + Send + Sync {
    fn read<'a>(
        &'a mut self,
    ) -> Pin<Box<dyn Future<Output = Result<Frame, FrameError>> + Send + 'a>>;
    fn is_connected<'a>(&'a self) -> Pin<Box<dyn Future<Output = bool> + Send + 'a>>;
}
