use std::future::Future;
use std::pin::Pin;

use tokio::io::{AsyncWrite, AsyncWriteExt, BufWriter};

use crate::{error::FrameError, pipeline::GameAction};

pub trait FramedWriter: Send + Sync {
    fn write(
        &mut self,
        action: GameAction,
    ) -> Pin<Box<dyn Future<Output = Result<(), FrameError>> + Send + '_>>;
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

impl<T: AsyncWrite + Unpin + Sync + Send> FramedWriter for FramedAsyncBufferedWriter<T> {
    fn write(
        &mut self,
        action: GameAction,
    ) -> Pin<Box<dyn Future<Output = Result<(), FrameError>> + Send + '_>> {
        Box::pin(async move {
            self.writer
                .write_all(&[action as u8])
                .await
                .map_err(|e| FrameError::Send(e.to_string()))?;
            Ok(())
        })
    }
}
