use crate::{Client, NetworkError};
use std::sync::Arc;
use tokio::{net::TcpListener, sync::Mutex};

#[derive(Debug, Clone)]
pub struct NetworkManager {
    pub clients: Vec<Client>,
    pub port: u16,
}

impl NetworkManager {
    pub fn new(port: u16) -> Self {
        Self {
            clients: Vec::new(),
            port,
        }
    }

    pub async fn start(&mut self) -> Result<(), NetworkError> {
        println!("Starting network manager on port {}", self.port);
        let listener = TcpListener::bind(format!("0.0.0.0:{}", self.port))
            .await
            .map_err(|e| NetworkError::BindError(e, self.port))?;

        loop {
            let (stream, _) = listener.accept().await.map_err(NetworkError::AcceptError)?;
            let client = Client::new(Arc::new(Mutex::new(stream)));
            println!(
                "New client connected: {:?}",
                client.stream.lock().await.peer_addr()
            );
            self.clients.push(client);
        }
    }

    pub async fn stop(&mut self) -> Result<(), NetworkError> {
        println!("Stopping network manager on port {}", self.port);
        for mut client in self.clients.drain(..) {
            client.stop().await.unwrap();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpStream;
    use tokio::time::Duration;

    const DEFAULT_PORT: u16 = 3344;

    #[tokio::test]
    async fn test_manager_new() {
        let mut manager = NetworkManager::new(DEFAULT_PORT);
        // share the manager with the test
        let mut manager_clone = manager.clone();
        tokio::spawn(async move {
            let result = manager_clone.start().await;
            assert!(result.is_ok());
        });
        tokio::time::sleep(Duration::from_secs(1)).await;
        let result = manager.stop().await;
    }

    #[tokio::test]
    async fn test_manager_start_with_port_and_client() {
        let mut manager = NetworkManager::new(12345);
        let mut manager_clone = manager.clone();
        tokio::spawn(async move {
            let result = manager_clone.start().await;
            assert!(result.is_ok());
        });
        tokio::time::sleep(Duration::from_secs(1)).await;
        let result = manager.stop().await;
        assert!(result.is_ok());
    }
}
