use crate::{
    error::AppError,
    network::{
        Frame,
        frame::frame_reader::FrameReader,
        frame_handler::{DelegatingRouter, PokemonFrameHandler},
    },
};
use std::sync::Arc;
use tokio::{
    net::TcpStream,
    sync::{
        Mutex,
        broadcast::{self, Sender},
        mpsc,
    },
};
use tracing::{debug, error, info};
use uuid::Uuid;

#[derive(Debug)]
pub struct Client {
    id: Uuid,
    reader: FrameReader,
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
    pub fn new(stream: TcpStream, pokemon_handler: PokemonFrameHandler) -> (Self, ClientHandle) {
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
            Self {
                id,
                reader: FrameReader::new(stream),
                shutdown_tx: shutdown_tx.clone(),
                frame_tx: tx,
            },
            ClientHandle { id, shutdown_tx },
        )
    }

    pub async fn handle_next_message(&mut self) -> Result<bool, AppError> {
        debug!("Handling next message for {:?}", self.id);
        if !self.is_connected().await {
            return Ok(false);
        }

        let frame = self.reader.read_frame().await?;
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
            tokio::select! {
                biased;
                _ = shutdown_rx.recv() => {
                    self.stop().await?;
                    debug!("Client pipeline for {:?} received shutdown", self.id);
                    break;
                }
                result = self.handle_next_message() => {
                    match result {
                        Ok(should_continue) => {
                            if !should_continue {
                                debug!("Client pipeline for {:?} received shutdown", self.id);
                                break;
                            }
                            debug!("Client pipeline for {:?} handled message", self.id);
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

    pub async fn stop(&mut self) -> Result<(), AppError> {
        info!("Shutting down client {:?}", self.id);
        self.reader
            .shutdown()
            .await
            .map_err(|e| AppError::Client(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::pipeline::FanoutService;

    use super::*;
    use std::{net::SocketAddr, time::Duration};
    use tokio::net::TcpListener;

    async fn setup_test_server(port: u16) -> (TcpListener, SocketAddr) {
        let listener = TcpListener::bind(format!("127.0.0.1:{}", port))
            .await
            .unwrap();
        let addr = listener.local_addr().unwrap();
        (listener, addr)
    }

    async fn cleanup_test_server(listener: TcpListener) {
        drop(listener);
    }

    #[tokio::test]
    async fn test_client_pipeline() {
        let (listener, addr) = setup_test_server(8080).await;

        let server_task = tokio::spawn(async move {
            let stream = TcpStream::connect(addr).await;
            assert!(stream.is_ok());
            let stream = stream.unwrap();
            let mut buf = [0; 1024];
            loop {
                match stream.try_read(&mut buf) {
                    Ok(_) => {
                        tokio::time::sleep(Duration::from_millis(100)).await;
                    }
                    Err(e) => {
                        if e.kind() == std::io::ErrorKind::WouldBlock {
                            tokio::time::sleep(Duration::from_millis(100)).await;
                        } else {
                            break;
                        }
                    }
                }
            }
        });

        let client = listener.accept().await;
        assert!(client.is_ok());
        let (stream, _) = client.unwrap();
        let (mut client, handle) = Client::new(
            stream,
            PokemonFrameHandler::new(FanoutService::new(10, vec![]).0),
        );
        let pipeline_task = tokio::spawn(async move {
            let result = client.run_pipeline().await;
            assert!(result.is_ok());
        });

        // warm up time for the client pipeline, may be racey
        tokio::time::sleep(Duration::from_millis(100)).await;

        let result = handle.send_shutdown().await;

        assert!(result.is_ok());
        pipeline_task.await.unwrap();
        server_task.abort();
        cleanup_test_server(listener).await;
    }
}
