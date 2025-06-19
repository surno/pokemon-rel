use crate::{
    error::AppError,
    intake::frame::{
        Frame,
        frame_handler::{DelegatingRouter, PokemonFrameHandler},
        iframe_reader::IFrameReader,
    },
};
use std::sync::Arc;
use tokio::sync::{
    Mutex,
    broadcast::{self, Sender},
    mpsc,
};
use tracing::{debug, error, info};
use uuid::Uuid;

#[derive(Debug)]
pub struct Client {
    id: Uuid,
    reader: Box<dyn IFrameReader + Send + Sync>,
    shutdown_tx: Sender<()>,
    frame_tx: mpsc::Sender<Frame>,
}

#[derive(Debug)]
pub struct ClientHandle {
    pub id: Uuid,
    shutdown_tx: Sender<()>,
}

impl ClientHandle {
    pub async fn send_shutdown(&self) -> Result<(), AppError> {
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
        pokemon_handler: PokemonFrameHandler,
        reader: Box<dyn IFrameReader + Send + Sync>,
    ) -> (Box<Client>, ClientHandle) {
        let (shutdown_tx, _) = broadcast::channel(1);
        let id = Uuid::new_v4();
        let (tx, mut rx) = mpsc::channel::<Frame>(1000);
        let router = Arc::new(Mutex::new(DelegatingRouter::new(pokemon_handler)));
        tokio::spawn(async move {
            while let Some(frame) = rx.recv().await {
                match router.try_lock() {
                    Ok(mut router) => {
                        let _ = router.route(&frame).await;
                    }
                    Err(e) => {
                        error!("Error locking router for {:?}: {:?}", id, e);
                    }
                }
            }
        });
        (
            Box::new(Client {
                id,
                reader,
                shutdown_tx: shutdown_tx.clone(),
                frame_tx: tx,
            }),
            ClientHandle { id, shutdown_tx },
        )
    }

    async fn handle_next_message(&mut self) -> Result<bool, AppError> {
        debug!("Handling next message for {:?}", self.id);
        if !self.is_connected().await {
            return Ok(false);
        }

        let frame = self.reader.read().await?;
        self.frame_tx.send(frame).await.map_err(|e| {
            error!("Error sending frame to client {:?}: {:?}", self.id, e);
            AppError::Client(e.to_string())
        })?;
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

    pub async fn is_connected(&self) -> bool {
        debug!("Checking if client {:?} is connected", self.id);
        self.reader.is_connected().await
    }
}
