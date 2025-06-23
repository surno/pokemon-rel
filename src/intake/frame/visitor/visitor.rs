use image::DynamicImage;
use tokio::sync::broadcast;

use crate::error::AppError;
use crate::intake::frame::Frame;
use crate::pipeline::EnrichedFrame;

pub trait FrameVisitor: Send + Sync {
    fn ping(&mut self) -> Result<(), AppError>;
    fn handshake(&mut self, version: u32, name: String, program: u16) -> Result<(), AppError>;
    fn image(&mut self, image: DynamicImage) -> Result<(), AppError>;
    fn shutdown(&mut self) -> Result<(), AppError>;
}

pub struct DelegatingRouter {
    visitor: Box<dyn FrameVisitor + Send + Sync>,
}

impl DelegatingRouter {
    pub fn new(visitor: Box<dyn FrameVisitor + Send + Sync>) -> Self {
        Self { visitor }
    }

    pub fn route(&mut self, frame: Frame) -> Result<(), AppError> {
        frame.accept(self.visitor.as_mut())
    }
}

pub struct FrameTranslatorVisitor {
    subscription: broadcast::Sender<EnrichedFrame>,
}

impl FrameVisitor for FrameTranslatorVisitor {
    fn ping(&mut self) -> Result<(), AppError> {
        Ok(())
    }
    fn handshake(&mut self, _: u32, _: String, _: u16) -> Result<(), AppError> {
        Ok(())
    }
    fn image(&mut self, _: DynamicImage) -> Result<(), AppError> {
        Ok(())
    }

    fn shutdown(&mut self) -> Result<(), AppError> {
        Ok(())
    }
}
