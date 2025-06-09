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
        stream
            .shutdown()
            .await
            .map_err(|e| NetworkError::ShutdownError(e.to_string()))?;
        self.is_connected = false;
        Ok(())
    }

    pub fn is_connected(&self) -> bool {
        self.is_connected
    }
}
