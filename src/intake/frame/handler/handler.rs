use image::DynamicImage;

use crate::error::AppError;
use crate::intake::frame::Frame;

pub trait FrameHandler: Send + Sync {
    fn handle_ping(&self) -> Result<(), AppError>;
    fn handle_handshake(&self, version: u32, name: String, program: u16) -> Result<(), AppError>;
    fn handle_image(&self, image: DynamicImage) -> Result<(), AppError>;
    fn handle_image_gd2(&self, width: u32, height: u32, gd2_data: Vec<u8>) -> Result<(), AppError>;
    fn handle_shutdown(&self) -> Result<(), AppError>;
}

pub struct DelegatingRouter {
    handler: Box<dyn FrameHandler + Send + Sync>,
}

impl DelegatingRouter {
    pub fn new(handler: Box<dyn FrameHandler + Send + Sync>) -> Self {
        Self { handler }
    }

    pub async fn route(&mut self, frame: Frame) -> Result<(), AppError> {
        match frame {
            Frame::Ping => self.handler.handle_ping(),
            Frame::Handshake {
                version,
                name,
                program,
            } => self
                .handler
                .handle_handshake(version, name.clone(), program),
            Frame::Image { image } => self.handler.handle_image(image),
            Frame::ImageGD2 {
                width,
                height,
                gd2_data,
            } => self.handler.handle_image_gd2(width, height, gd2_data),
            Frame::Shutdown => self.handler.handle_shutdown(),
        }
    }
}
