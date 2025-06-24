use tokio::io::{AsyncWrite, BufWriter};

use crate::{error::FrameError, intake::frame::Frame};

pub trait FrameWriter: Send + Sync {
    fn write(&mut self, frame: Frame) -> Result<(), FrameError>;
}

pub struct FramedAsyncBufferedWriter<T>
where
    T: AsyncWrite + Unpin + Sync + Send,
{
    writer: BufWriter<T>,
}

impl<T: AsyncWrite + Unpin + Sync + Send> FramedAsyncBufferedWriter<T> {
    pub fn new(writer: T) -> Self {
        Self {
            writer: BufWriter::new(writer),
        }
    }
}

impl<T: AsyncWrite + Unpin + Sync + Send> FrameWriter for FramedAsyncBufferedWriter<T> {
    fn write(&mut self, frame: Frame) -> Result<(), FrameError> {
        Ok(())
    }
}
