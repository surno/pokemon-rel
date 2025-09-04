use image::DynamicImage;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::error::AppError;
use crate::intake::frame::writer::FramedWriter;
use crate::pipeline::{EnrichedFrame, GameAction};

pub struct AIFrameVisitor {
    frame_tx: mpsc::Sender<EnrichedFrame>,
    action_rx: mpsc::Receiver<GameAction>,
    writer: Box<dyn FramedWriter + Send + Sync>,
    state: ClientState,
    client_id: Uuid,
    program: u16,
}

#[derive(PartialEq)]
enum ClientState {
    Handshake,
    Running,
    Shutdown,
}

impl AIFrameVisitor {
    pub fn new(
        frame_tx: mpsc::Sender<EnrichedFrame>,
        action_rx: mpsc::Receiver<GameAction>,
        writer: Box<dyn FramedWriter + Send + Sync>,
    ) -> Self {
        Self {
            frame_tx,
            action_rx,
            writer,
            state: ClientState::Handshake,
            client_id: Uuid::new_v4(),
            program: 0,
        }
    }

    pub async fn process_actions(&mut self) -> Result<(), AppError> {
        while let Ok(action) = self.action_rx.try_recv() {
            // Here you would send the action to the emulator
            // For now, we'll just log it
            tracing::info!(
                "AI decided action for client {}: {:?}",
                self.client_id,
                action
            );

            // Send action to emulator via the writer
            self.writer.send_action(action).await?;
        }
        Ok(())
    }
}

impl super::FrameVisitor for AIFrameVisitor {
    fn ping(&mut self) -> Result<(), AppError> {
        Ok(())
    }

    fn handshake(&mut self, id: Uuid, program: u16) -> Result<(), AppError> {
        match self.state {
            ClientState::Handshake => {
                self.state = ClientState::Running;
                self.client_id = id;
                self.program = program;
                tracing::info!("AI Frame Visitor handshake completed for client {}", id);
                Ok(())
            }
            _ => Err(AppError::Client("Client already connected".to_string())),
        }
    }

    fn image(&mut self, image: DynamicImage) -> Result<(), AppError> {
        if self.state == ClientState::Running || self.state == ClientState::Handshake {
            // Create enriched frame and send to AI pipeline
            let enriched_frame = EnrichedFrame::new(self.client_id, image, self.program);

            if let Err(e) = self.frame_tx.try_send(enriched_frame) {
                tracing::warn!("Failed to send frame to AI pipeline: {}", e);
            }

            // Note: Actions will be processed by the AI pipeline service
            // We can't process them here due to trait constraints

            Ok(())
        } else {
            Err(AppError::Client("Client is not available.".to_string()))
        }
    }

    fn shutdown(&mut self) -> Result<(), AppError> {
        self.state = ClientState::Shutdown;
        tracing::info!("AI Frame Visitor shutdown for client {}", self.client_id);
        Ok(())
    }
}
