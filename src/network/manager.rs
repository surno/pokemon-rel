use crate::Client;
use std::sync::Arc;
use tokio::net::TcpListener;

const DEFAULT_PORT: u16 = 3344;

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

    async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Starting network manager on port {}", self.port);
        let listener = TcpListener::bind(format!("0.0.0.0:{}", self.port))
            .await;
        match listener {
            Ok(listener) => {
                println!("Listening on port {}", self.port);
            }
            
             
        }

        for stream in listener.accept() {
            match stream {
                Ok(stream) => {
                    let client = Client::new(Arc::new(stream));
                    self.clients.push(client);
                }
                Err(e) => {
                    println!("Error accepting connection: {}", e);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpStream;

    #[test]
    fn test_manager_new() {
        let manager = NetworkManager::new();
        manager.start();
    }

    #[test]
    fn test_manager_start() {
        let manager = NetworkManager::new();
        manager.start();
    }

    #[test]
    fn test_manager_start_with_port() {
        let manager = NetworkManager::new(12345);
        manager.start();
    }

    #[test]
    fn test_manager_start_with_port_and_client() {
        let manager = NetworkManager::new(12345);
        manager.start();
        let client = Client::new(Arc::new(TcpStream::connect("127.0.0.1:12345").unwrap()));
        assert!(client.is_connected);
    }
}
