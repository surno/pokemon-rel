use crate::error::FrameError;
use crate::network::frame::Frame;
use std::fmt::Debug;

pub trait FrameHandler: Send + Sync + 'static + Debug {
    fn handle_ping(&self) -> Result<(), FrameError>;
    fn handle_handshake(&self, version: u32, name: String, program: u16) -> Result<(), FrameError>;
    fn handle_image(&mut self, width: u32, height: u32, pixels: Vec<u8>) -> Result<(), FrameError>;
    fn handle_shutdown(&self) -> Result<(), FrameError>;
}

pub struct DelegatingRouter<H: FrameHandler> {
    handler: H,
}

impl<H: FrameHandler> DelegatingRouter<H> {
    pub async fn route(&mut self, data: &[u8]) -> Result<(), FrameError> {
        let frame = Frame::try_from(data)?;
        match frame {
            Frame::Ping => self.handler.handle_ping(),
            Frame::Handshake {
                version,
                name,
                program,
            } => self.handler.handle_handshake(version, name, program),
            Frame::Image {
                width,
                height,
                pixels,
            } => self.handler.handle_image(width, height, pixels),
            Frame::Shutdown => self.handler.handle_shutdown(),
        }
    }
}
