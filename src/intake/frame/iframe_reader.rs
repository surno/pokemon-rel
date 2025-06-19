use crate::{error::FrameError, intake::frame::Frame};

#[derive(Debug)]
pub enum ReadState {
    WaitingForLength,
    WaitingForFrame { expected_length: u32 },
}

pub trait IFrameReader {
    fn read(&mut self) -> impl std::future::Future<Output = Result<Frame, FrameError>> + Send;
    fn is_connected(&self) -> impl std::future::Future<Output = bool> + Send;
}
