use crate::error::AppError;
use crate::intake::frame::Frame;
use std::future::Future;
use std::pin::Pin;

pub trait FrameHandler: Send + Sync {
    fn handle_ping<'a>(&'a self)
    -> Pin<Box<dyn Future<Output = Result<(), AppError>> + Send + 'a>>;
    fn handle_handshake<'a>(
        &'a self,
        version: u32,
        name: String,
        program: u16,
    ) -> Pin<Box<dyn Future<Output = Result<(), AppError>> + Send + 'a>>;
    fn handle_image<'a>(
        &'a self,
        width: u32,
        height: u32,
        pixels: Vec<u8>,
    ) -> Pin<Box<dyn Future<Output = Result<(), AppError>> + Send + 'a>>;
    fn handle_image_gd2<'a>(
        &'a self,
        width: u32,
        height: u32,
        pixels: Vec<u8>,
    ) -> Pin<Box<dyn Future<Output = Result<(), AppError>> + Send + 'a>>;
    fn handle_shutdown<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = Result<(), AppError>> + Send + 'a>>;
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
            Frame::Ping => self.handler.handle_ping().await,
            Frame::Handshake {
                version,
                name,
                program,
            } => {
                self.handler
                    .handle_handshake(version, name.clone(), program)
                    .await
            }
            Frame::Image {
                width,
                height,
                pixels,
            } => {
                self.handler
                    .handle_image(width, height, pixels.clone())
                    .await
            }
            Frame::ImageGD2 {
                width,
                height,
                gd2_data,
            } => {
                self.handler
                    .handle_image_gd2(width, height, gd2_data.clone())
                    .await
            }
            Frame::Shutdown => self.handler.handle_shutdown().await,
        }
    }
}
