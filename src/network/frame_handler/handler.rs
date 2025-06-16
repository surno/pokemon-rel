use crate::error::AppError;
use crate::network::frame::Frame;
use std::fmt::Debug;
use std::future::Future;

pub trait FrameHandler: Send + Sync + 'static + Debug {
    fn handle_ping(&self) -> impl Future<Output = Result<(), AppError>>;
    fn handle_handshake(
        &self,
        version: u32,
        name: String,
        program: u16,
    ) -> impl Future<Output = Result<(), AppError>>;
    fn handle_image(
        &mut self,
        width: u32,
        height: u32,
        pixels: Vec<u8>,
    ) -> impl Future<Output = Result<(), AppError>>;
    fn handle_image_gd2(
        &mut self,
        width: u32,
        height: u32,
        gd2_data: Vec<u8>,
    ) -> impl Future<Output = Result<(), AppError>>;
    fn handle_shutdown(&self) -> impl Future<Output = Result<(), AppError>>;
}

#[derive(Debug)]
pub struct DelegatingRouter<H: FrameHandler> {
    handler: H,
}

impl<H: FrameHandler> DelegatingRouter<H> {
    pub fn new(handler: H) -> Self {
        Self { handler }
    }

    pub async fn route(&mut self, frame: &Frame) -> Result<(), AppError> {
        match frame {
            Frame::Ping => self.handler.handle_ping().await,
            Frame::Handshake {
                version,
                name,
                program,
            } => {
                self.handler
                    .handle_handshake(*version, name.clone(), *program)
                    .await
            }
            Frame::Image {
                width,
                height,
                pixels,
            } => {
                self.handler
                    .handle_image(*width, *height, pixels.clone())
                    .await
            }
            Frame::ImageGD2 {
                width,
                height,
                gd2_data,
            } => {
                self.handler
                    .handle_image_gd2(*width, *height, gd2_data.clone())
                    .await
            }
            Frame::Shutdown => self.handler.handle_shutdown().await,
        }
    }
}
