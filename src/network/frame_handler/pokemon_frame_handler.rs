use crate::error::FrameError;
use crate::network::frame_handler::FrameHandler;
use crate::pipeline::services::FanoutService;
use crate::pipeline::types::RawFrame;
use tower::Service;
use tracing::debug;

#[derive(Debug, Clone)]
pub struct PokemonFrameHandler {
    fanout_service: FanoutService,
}

impl PokemonFrameHandler {
    pub fn new(fanout_service: FanoutService) -> Self {
        Self { fanout_service }
    }
}

impl FrameHandler for PokemonFrameHandler {
    fn handle_ping(&self) -> Result<(), FrameError> {
        debug!("Received ping");
        Ok(())
    }

    fn handle_handshake(&self, version: u32, name: String, program: u16) -> Result<(), FrameError> {
        debug!(
            "Received handshake: version={}, name={}, program={}",
            version, name, program
        );
        Ok(())
    }

    fn handle_image(&mut self, width: u32, height: u32, pixels: Vec<u8>) -> Result<(), FrameError> {
        debug!("Received image: width={}, height={}", width, height);
        let raw_frame = RawFrame::new(width, height, pixels);
        let _result = self.fanout_service.call(raw_frame);
        Ok(())
    }

    fn handle_shutdown(&self) -> Result<(), FrameError> {
        debug!("Received shutdown");
        Ok(())
    }
}
