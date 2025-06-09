use crate::{Client, NetworkError};
use std::sync::Arc;
use tokio::{net::TcpListener, sync::Mutex, sync::RwLock, sync::broadcast};

#[derive(Debug)]
pub struct NetworkManager {
    clients: Arc<RwLock<Vec<Client>>>,
    port: u16,
    shutdown_tx: broadcast::Sender<()>,
    listener: Option<TcpListener>,
}

#[derive(Debug, Clone)]
pub struct NetworkHandle {
    shutdown_tx: broadcast::Sender<()>,
    clients: Arc<RwLock<Vec<Client>>>,
}

impl NetworkHandle {
    pub async fn shutdown(&self) -> Result<(), NetworkError> {
        self.shutdown_tx
            .send(())
            .map_err(|e| NetworkError::ShutdownError(e.to_string()))?;
        Ok(())
    }

    pub async fn get_client_count(&self) -> usize {
        self.clients.read().await.len()
    }
}

impl NetworkManager {
    pub fn new(port: u16) -> (Self, NetworkHandle) {
        let (shutdown_tx, _) = broadcast::channel(1);
        let clients = Arc::new(RwLock::new(Vec::new()));
        (
            Self {
                clients: clients.clone(),
                port,
                shutdown_tx: shutdown_tx.clone(),
                listener: None,
            },
            NetworkHandle {
                shutdown_tx,
                clients,
            },
        )
    }

    pub async fn start(&mut self) -> Result<(), NetworkError> {
        println!("Starting network manager on port {}", self.port);
        let mut shutdown_rx = self.shutdown_tx.subscribe();
        self.listener = Some(
            TcpListener::bind(format!("0.0.0.0:{}", self.port))
                .await
                .map_err(|e| NetworkError::BindError(e, self.port))?,
        );
        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    println!("Shutting down network manager.");
                    self.shutdown().await;
                    return Ok(());
                }
                result = self.listener.as_ref().unwrap().accept() => {
                    if let Ok((stream, _)) = result {
                        println!("New client connected: {:?}", stream.peer_addr());
                        let client = Client::new(Arc::new(Mutex::new(stream)));
                        self.clients.write().await.push(client);
                    } else {
                        println!("Error accepting connection: {:?}", result.err());
                    }
                }
            };
        }
    }

    pub async fn shutdown(&mut self) {
        println!("Stopping network manager on port {}", self.port);
        for mut client in self.clients.write().await.drain(..) {
            let result = client.stop().await;
            match result {
                Ok(_) => println!(
                    "Client disconnected: {:?}",
                    client.stream.lock().await.peer_addr()
                ),
                Err(e) => println!("Error stopping client: {:?}", e),
            }
        }
        self.listener = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::Duration;

    const DEFAULT_PORT: u16 = 0;

    #[tokio::test]
    async fn test_manager_new() {
        let (mut manager, handle) = NetworkManager::new(DEFAULT_PORT);
        // share the manager with the test
        tokio::spawn(async move {
            let result = manager.start().await;
            assert!(result.is_ok());
        });
        tokio::time::sleep(Duration::from_secs(1)).await;
        assert!(handle.shutdown().await.is_ok());
    }

    #[tokio::test]
    async fn test_manager_start_and_shutdown() {
        let (mut manager, handle) = NetworkManager::new(DEFAULT_PORT);
        tokio::spawn(async move {
            let result = manager.start().await;
            assert!(
                result.is_ok(),
                "Failed to start manager: {:?}",
                result.err()
            );
        });
        tokio::time::sleep(Duration::from_secs(1)).await;
        assert!(handle.shutdown().await.is_ok());
    }

    #[tokio::test]
    async fn test_manager_get_client_count() {
        let (mut manager, handle) = NetworkManager::new(DEFAULT_PORT);
        tokio::spawn(async move {
            let result = manager.start().await;
            assert!(result.is_ok());
        });
        tokio::time::sleep(Duration::from_secs(1)).await;
        assert_eq!(handle.get_client_count().await, 0);
    }
}
