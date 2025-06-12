use crate::error::FrameError;
use crate::network::frame::Frame;
use std::fmt::Debug;

pub trait FrameHandler: Send + Sync + 'static + Debug {
    fn handle_ping(&self) -> Result<(), FrameError>;
    fn handle_handshake(&self, version: u32, name: String, program: u16) -> Result<(), FrameError>;
    fn handle_image(&mut self, width: u32, height: u32, pixels: Vec<u8>) -> Result<(), FrameError>;
    fn handle_image_gd2(
        &mut self,
        width: u32,
        height: u32,
        gd2_data: Vec<u8>,
    ) -> Result<(), FrameError>;
    fn handle_shutdown(&self) -> Result<(), FrameError>;
}

#[derive(Debug)]
pub struct DelegatingRouter<H: FrameHandler> {
    handler: H,
}

impl<H: FrameHandler> DelegatingRouter<H> {
    pub fn new(handler: H) -> Self {
        Self { handler }
    }

    pub async fn route(&mut self, frame: &Frame) -> Result<(), FrameError> {
        match frame {
            Frame::Ping => self.handler.handle_ping(),
            Frame::Handshake {
                version,
                name,
                program,
            } => self
                .handler
                .handle_handshake(*version, name.clone(), *program),
            Frame::Image {
                width,
                height,
                pixels,
            } => self.handler.handle_image(*width, *height, pixels.clone()),
            Frame::ImageGD2 {
                width,
                height,
                gd2_data,
            } => self
                .handler
                .handle_image_gd2(*width, *height, gd2_data.clone()),
            Frame::Shutdown => self.handler.handle_shutdown(),
        }
    }
}
