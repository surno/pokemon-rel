use crate::{
    error::AppError,
    intake::frame::{
        Frame,
        handler::{DelegatingRouter, FrameHandler},
        reader::FrameReader,
    },
};
use tokio::sync::{
    broadcast::{self, Sender},
    mpsc,
};
use tracing::{debug, error, info};
use uuid::Uuid;

pub struct Client {
    id: Uuid,
    reader: Box<dyn FrameReader + Send + Sync>,
    shutdown_tx: Sender<()>,
    frame_tx: mpsc::Sender<Frame>,
    router: DelegatingRouter,
}

#[derive(Debug)]
pub struct ClientHandle {
    pub id: Uuid,
    shutdown_tx: Sender<()>,
}

impl ClientHandle {
    pub fn send_shutdown(&self) -> Result<(), AppError> {
        match self.shutdown_tx.send(()) {
            Ok(_) => Ok(()),
            Err(e) => {
                error!(
                    "Error sending shutdown to client handle {:?}: {:?}",
                    self.id,
                    e.to_string()
                );
                Err(AppError::ClientShutdown(self.id))
            }
        }
    }
}

impl Client {
    pub fn new(
        handler: Box<dyn FrameHandler + Send + Sync>,
        reader: Box<dyn FrameReader + Send + Sync>,
    ) -> (Box<Client>, ClientHandle) {
        let (shutdown_tx, _) = broadcast::channel(1);
        let id = Uuid::new_v4();
        let (tx, _) = mpsc::channel::<Frame>(1000);
        let router = DelegatingRouter::new(handler);
        (
            Box::new(Client {
                id,
                reader,
                shutdown_tx: shutdown_tx.clone(),
                frame_tx: tx,
                router: router,
            }),
            ClientHandle { id, shutdown_tx },
        )
    }

    async fn handle_next_message(&mut self) -> Result<bool, AppError> {
        debug!("Handling next message for {:?}", self.id);
        let frame = self.reader.read().await?;
        self.router.route(frame)?;
        Ok(true)
    }

    pub async fn run_pipeline(&mut self) -> Result<(), AppError> {
        info!("Running client pipeline for {:?}", self.id);
        let mut shutdown_rx = self.shutdown_tx.subscribe();
        loop {
            let next_message = self.handle_next_message();
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    debug!("Client pipeline for {:?} received shutdown", self.id);
                    break;
                }
                result = next_message => {
                    match result {
                        Ok(true) => {
                            debug!("Client pipeline for {:?} handled message", self.id);
                        }
                        Ok(false) => {
                            debug!("Client {:?} has disconnected", self.id);
                            break;
                        }
                        Err(e) => {
                            error!("Client pipeline for {:?} handled message: {:?}", self.id, e);
                            return Err(e);
                        }
                    }
                }
            }
        }
        debug!("Client pipeline for {:?} finished", self.id);
        Ok(())
    }

    pub fn id(&self) -> Uuid {
        self.id
    }
}
