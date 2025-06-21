use crate::{error::FrameError, intake::frame::Frame};
use std::future::Future;
use std::pin::Pin;

pub enum ReadState {
    WaitingForLength,
    WaitingForFrame { expected_length: u32 },
}

pub trait FrameReader: Send + Sync {
    fn read_frame_length<'a>(
        &'a mut self,
    ) -> Pin<Box<dyn Future<Output = Result<u32, FrameError>> + Send + 'a>>;
    fn read_frame_data<'a>(
        &'a mut self,
        expected_length: u32,
    ) -> Pin<Box<dyn Future<Output = Result<Frame, FrameError>> + Send + 'a>>;
    fn read<'a>(
        &'a mut self,
    ) -> Pin<Box<dyn Future<Output = Result<Frame, FrameError>> + Send + 'a>> {
        Box::pin(async move {
            let mut state = ReadState::WaitingForLength;
            loop {
                match &mut state {
                    ReadState::WaitingForLength => {
                        state = ReadState::WaitingForFrame {
                            expected_length: self.read_frame_length().await?,
                        };
                    }
                    ReadState::WaitingForFrame { expected_length } => {
                        return self.read_frame_data(*expected_length).await;
                    }
                }
            }
        })
    }
}
