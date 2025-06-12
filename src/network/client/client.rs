use crate::{
    ClientError, NetworkError,
    network::frame::frame_reader::FrameReader,
    network::frame_handler::{DelegatingRouter, PokemonFrameHandler},
};
use tokio::{
    net::TcpStream,
    sync::broadcast::{self, Sender},
};
use tracing::{debug, error, info};
use uuid::Uuid;

#[derive(Debug)]
pub struct Client {
    id: Uuid,
    reader: FrameReader,
    shutdown_tx: Sender<()>,
    router: DelegatingRouter<PokemonFrameHandler>,
}

#[derive(Debug)]
pub struct ClientHandle {
    pub id: Uuid,
    shutdown_tx: Sender<()>,
}

impl ClientHandle {
    pub async fn send_shutdown(&self) -> Result<(), ClientError> {
        match self.shutdown_tx.send(()) {
            Ok(_) => Ok(()),
            Err(e) => {
                error!(
                    "Error sending shutdown to client handle {:?}: {:?}",
                    self.id,
                    e.to_string()
                );
                Err(ClientError::ShutdownError(self.id))
            }
        }
    }
}

impl Client {
    pub fn new(stream: TcpStream, pokemon_handler: PokemonFrameHandler) -> (Self, ClientHandle) {
        let (shutdown_tx, _) = broadcast::channel(1);
        let id = Uuid::new_v4();
        (
            Self {
                id,
                reader: FrameReader::new(stream),
                shutdown_tx: shutdown_tx.clone(),
                router: DelegatingRouter::new(pokemon_handler),
            },
            ClientHandle { id, shutdown_tx },
        )
    }

    pub async fn handle_next_message(&mut self) -> Result<bool, ClientError> {
        debug!("Handling next message for {:?}", self.id);
        if !self.is_connected().await {
            return Ok(false);
        }

        match self.reader.read_frame().await {
            Ok(frame) => {
                // Frame received (verbose logging removed)
                self.router
                    .route(&frame)
                    .await
                    .map_err(|e| ClientError::RouteError(e))?;
                Ok(true)
            }
            Err(e) => Err(ClientError::ReadError(e)),
        }
    }

    pub async fn run_pipeline(&mut self) -> Result<(), ClientError> {
        info!("Running client pipeline for {:?}", self.id);
        let mut shutdown_rx = self.shutdown_tx.subscribe();
        loop {
            tokio::select! {
                biased;
                _ = shutdown_rx.recv() => {
                    self.stop().await.map_err(|e: NetworkError| ClientError::StopError(e))?;
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

    pub async fn stop(&mut self) -> Result<(), NetworkError> {
        info!("Shutting down client {:?}", self.id);
        self.reader
            .shutdown()
            .await
            .map_err(|e| NetworkError::ShutdownError(e.to_string()))?;
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
        let (mut client, handle) =
            Client::new(stream, PokemonFrameHandler::new(FanoutService::new(10).0));
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
