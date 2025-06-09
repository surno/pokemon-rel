use crate::{Client, NetworkError};
use std::sync::Arc;
use tokio::{net::TcpListener, sync::Mutex, sync::broadcast};

#[derive(Debug)]
pub struct NetworkManager {
    clients: Vec<Client>,
    port: u16,
    shutdown_rx: broadcast::Receiver<()>,
}

#[derive(Debug)]
pub struct NetworkHandle {
    shutdown_tx: broadcast::Sender<()>,
}

impl NetworkHandle {
    pub async fn shutdown(&self) -> Result<(), NetworkError> {
        self.shutdown_tx
            .send(())
            .map_err(|e| NetworkError::ShutdownError(e.to_string()))?;
        Ok(())
    }
}

impl NetworkManager {
    pub fn new(port: u16) -> (Self, NetworkHandle) {
        let (shutdown_tx, shutdown_rx) = broadcast::channel(1);
        (
            Self {
                clients: Vec::new(),
                port,
                shutdown_rx,
            },
            NetworkHandle { shutdown_tx },
        )
    }

    pub async fn start(&mut self) -> Result<(), NetworkError> {
        println!("Starting network manager on port {}", self.port);
        let listener = TcpListener::bind(format!("0.0.0.0:{}", self.port))
            .await
            .map_err(|e| NetworkError::BindError(e, self.port))?;

        loop {
            tokio::select! {
                _ = self.shutdown_rx.recv() => {
                    println!("Shutting down network manager.");
                    self.shutdown().await;
                    return Ok(());
                }
                result = listener.accept() => {
                    if let Ok((stream, _)) = result {
                        println!("New client connected: {:?}", stream.peer_addr());
                        let client = Client::new(Arc::new(Mutex::new(stream)));
                        self.clients.push(client);
                    } else {
                        println!("Error accepting connection: {:?}", result.err());
                    }
                }
            };
        }
    }

    pub async fn shutdown(&mut self) {
        println!("Stopping network manager on port {}", self.port);
        for mut client in self.clients.drain(..) {
            let result = client.stop().await;
            match result {
                Ok(_) => println!(
                    "Client disconnected: {:?}",
                    client.stream.lock().await.peer_addr()
                ),
                Err(e) => println!("Error stopping client: {:?}", e),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::Duration;

    const DEFAULT_PORT: u16 = 3344;

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
        let (mut manager, handle) = NetworkManager::new(12345);
        tokio::spawn(async move {
            let result = manager.start().await;
            assert!(result.is_ok());
        });
        tokio::time::sleep(Duration::from_secs(1)).await;
        assert!(handle.shutdown().await.is_ok());
    }
}
