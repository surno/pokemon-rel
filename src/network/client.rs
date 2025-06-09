use crate::NetworkError;
use std::sync::Arc;
use tokio::{io::AsyncWriteExt, net::TcpStream, sync::Mutex};

#[derive(Debug, Clone)]
pub struct Client {
    pub stream: Arc<Mutex<TcpStream>>,
    pub is_connected: bool,
}

impl Client {
    pub fn new(stream: Arc<Mutex<TcpStream>>) -> Self {
        Self {
            stream,
            is_connected: true,
        }
    }

    pub async fn stop(&mut self) -> Result<(), NetworkError> {
        let mut stream = self.stream.lock().await;
        let result = stream.shutdown().await.map_err(NetworkError::ShutdownError);
        self.is_connected = false;
        result
    }

    pub fn is_connected(&self) -> bool {
        self.is_connected
    }
}
