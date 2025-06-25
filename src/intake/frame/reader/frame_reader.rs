use crate::{error::FrameError, intake::frame::Frame};
use std::future::Future;
use std::pin::Pin;

pub trait FrameReader: Send + Sync {
    fn read<'a>(
        &'a mut self,
    ) -> Pin<Box<dyn Future<Output = Result<Frame, FrameError>> + Send + 'a>>;
}
