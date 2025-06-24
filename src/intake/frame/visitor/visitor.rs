use image::DynamicImage;
use tokio::sync::mpsc;
use tracing::debug;
use uuid::Uuid;

use crate::error::AppError;
use crate::pipeline::EnrichedFrame;

pub trait FrameVisitor: Send + Sync {
    fn ping(&mut self) -> Result<(), AppError>;
    fn handshake(&mut self, id: Uuid, program: u16) -> Result<(), AppError>;
    fn image(&mut self, image: DynamicImage) -> Result<(), AppError>;
    fn shutdown(&mut self) -> Result<(), AppError>;
}

#[derive(PartialEq)]
enum ClientState {
    Handshake,
    Running,
    Shutdown,
}
pub struct FrameDelegatingVisitor {
    subscription: mpsc::Sender<EnrichedFrame>,
    state: ClientState,
    client_id: Uuid,
    program: u16,
}

impl FrameDelegatingVisitor {
    pub fn new(subscription: mpsc::Sender<EnrichedFrame>) -> Self {
        Self {
            subscription,
            state: ClientState::Handshake,
            client_id: Uuid::new_v4(),
            program: 0,
        }
    }
}

impl FrameVisitor for FrameDelegatingVisitor {
    fn ping(&mut self) -> Result<(), AppError> {
        Ok(())
    }
    fn handshake(&mut self, id: Uuid, program: u16) -> Result<(), AppError> {
        match self.state {
            ClientState::Handshake => {
                self.state = ClientState::Running;
                self.client_id = id;
                self.program = program;
                Ok(())
            }
            _ => Err(AppError::Client("Client already connected".to_string())),
        }
    }
    fn image(&mut self, image: DynamicImage) -> Result<(), AppError> {
        if self.state == ClientState::Running || self.state == ClientState::Handshake {
            // send the enriched frame to the subscription
            debug!(
                "Visitor sending frame to subscription: {:?}",
                self.client_id
            );
            self.subscription
                .try_send(EnrichedFrame::new(self.client_id, image, self.program))
                .map_err(|e| AppError::Client(e.to_string()))?;
            Ok(())
        } else {
            Err(AppError::Client("Client is not available.".to_string()))
        }
    }

    fn shutdown(&mut self) -> Result<(), AppError> {
        self.state = ClientState::Shutdown;
        Ok(())
    }
}
