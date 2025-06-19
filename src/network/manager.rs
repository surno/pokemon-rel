use crate::intake::{
    client::client_manager::{ClientManagerTrait, FrameReaderClientManager},
    frame::{
        frame_handler::PokemonFrameHandler, frame_reader::FramedTcpReader,
        iframe_reader::IFrameReader,
    },
};
use crate::pipeline::services::{FanoutService, preprocessing::FrameHashingBuilder};
use crate::pipeline::types::GameState;
use crate::{
    error::AppError,
    intake::{client::Client, client::ClientHandle},
};
use std::fs;
use std::sync::Arc;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{RwLock, broadcast},
};

use tracing::{error, info};

#[derive(Debug)]
pub struct NetworkManager {
    client_handles: Arc<RwLock<Vec<ClientHandle>>>,
    client_manager: Arc<RwLock<FrameReaderClientManager>>,
    port: u16,
    shutdown_tx: broadcast::Sender<()>,
    listener: Option<TcpListener>,
}

#[derive(Debug)]
pub struct NetworkHandle {
    shutdown_tx: broadcast::Sender<()>,
    client_handles: Arc<RwLock<Vec<ClientHandle>>>,
}

impl NetworkHandle {
    pub async fn shutdown(&self) -> Result<(), AppError> {
        self.shutdown_tx
            .send(())
            .map_err(|e| AppError::Client(e.to_string()))?;
        Ok(())
    }

    pub async fn get_client_count(&self) -> usize {
        self.client_handles.read().await.len()
    }
}

impl NetworkManager {
    pub fn new(
        port: u16,
        client_manager: Arc<RwLock<FrameReaderClientManager>>,
    ) -> (Self, NetworkHandle) {
        let (shutdown_tx, _) = broadcast::channel(1);
        let client_handles = Arc::new(RwLock::new(Vec::new()));
        (
            Self {
                client_handles: client_handles.clone(),
                client_manager,
                port,
                shutdown_tx: shutdown_tx.clone(),
                listener: None,
            },
            NetworkHandle {
                shutdown_tx,
                client_handles,
            },
        )
    }

    pub async fn start(&mut self) -> Result<(), AppError> {
        info!("Starting network manager on port {}", self.port);
        if self.listener.is_some() {
            return Err(AppError::AlreadyStarted);
        }
        self.listener = Some(
            TcpListener::bind(format!("0.0.0.0:{}", self.port))
                .await
                .map_err(|e| AppError::Bind(e, self.port))?,
        );
        let mut shutdown_rx = self.shutdown_tx.subscribe();
        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    info!("Shutting down network manager.");
                    self.shutdown().await;
                    return Ok(());
                }
                result = self.listener.as_ref().unwrap().accept() => {
                    if let Ok((stream, _)) = result {
                        self.spawn_client_pipeline(stream).await;
                    } else {
                        error!("Error accepting connection: {:?}", result.err());
                    }
                }
            };
        }
    }

    pub async fn spawn_client_pipeline(&self, stream: TcpStream) {
        let addr = stream.peer_addr().unwrap();
        let intro_hashes = fs::read_to_string("./assets/intro_hashes.txt").unwrap();
        let main_menu_hashes = fs::read_to_string("./assets/main_menu_hashes.txt").unwrap();

        let frame_hashing_service = FrameHashingBuilder::new(1000, 0.1)
            .with_game_state(
                GameState::Intro,
                intro_hashes.lines().map(|line| line.to_string()).collect(),
            )
            .with_game_state(
                GameState::MainMenu,
                main_menu_hashes
                    .lines()
                    .map(|line| line.to_string())
                    .collect(),
            )
            .build();

        let (visualization_tx, _visualization_rx) = broadcast::channel(10);
        let fanout_service = FanoutService::new(frame_hashing_service, visualization_tx);

        let pokemon_handler = PokemonFrameHandler::new(fanout_service);

        let reader: Box<dyn IFrameReader + Send + Sync> = Box::new(FramedTcpReader::new(stream));

        let (client, client_handle) = Client::new(pokemon_handler, reader);
        let client_id = client.id();

        info!(
            "New client attempting to connect: {:?} from {:?}",
            client_id, addr
        );

        self.client_handles.write().await.push(client_handle);

        let client_manager = self.client_manager.clone();
        let mut client_manager = client_manager.write().await;
        client_manager.add_client(client);
        info!("Client connected: {:?} from {:?}", client_id, addr);
    }

    pub async fn shutdown(&mut self) {
        info!("Stopping network manager on port {}", self.port);
        for client_handle in self.client_handles.write().await.drain(..) {
            let result = client_handle.send_shutdown().await;
            match result {
                Ok(_) => info!("Client disconnected: {:?}", client_handle.id),
                Err(e) => error!("Error stopping client: {:?}", e),
            }
        }
        match self.listener.take() {
            Some(listener) => {
                drop(listener);
            }
            None => {
                error!("No listener to shutdown");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::Duration;

    const DEFAULT_PORT: u16 = 0;

    #[tokio::test]
    async fn test_manager_new() {
        let client_manager = Arc::new(RwLock::new(FrameReaderClientManager::new()));
        let (mut manager, handle) = NetworkManager::new(DEFAULT_PORT, client_manager);
        // share the manager with the test
        tokio::spawn(async move {
            let _ = manager.start().await;
        });
        tokio::time::sleep(Duration::from_secs(1)).await;
        assert!(handle.shutdown().await.is_ok());
    }

    #[tokio::test]
    async fn test_manager_start_and_shutdown() {
        let client_manager = Arc::new(RwLock::new(FrameReaderClientManager::new()));
        let (mut manager, handle) = NetworkManager::new(DEFAULT_PORT, client_manager);
        tokio::spawn(async move {
            let result = manager.start().await;
        });
        tokio::time::sleep(Duration::from_secs(1)).await;
        assert!(handle.shutdown().await.is_ok());
    }

    #[tokio::test]
    async fn test_manager_get_client_count() {
        let client_manager = Arc::new(RwLock::new(FrameReaderClientManager::new()));
        let (mut manager, handle) = NetworkManager::new(DEFAULT_PORT, client_manager);
        tokio::spawn(async move {
            let result = manager.start().await;
        });
        tokio::time::sleep(Duration::from_secs(1)).await;
        assert_eq!(handle.get_client_count().await, 0);
    }

    #[tokio::test]
    async fn test_manager_start_and_shutdown_twice() {
        let client_manager = Arc::new(RwLock::new(FrameReaderClientManager::new()));
        let (mut manager, _) = NetworkManager::new(DEFAULT_PORT, client_manager);
        tokio::spawn(async move {
            let _ = manager.start().await;
        });
    }
}
