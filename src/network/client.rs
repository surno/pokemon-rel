use crate::{ClientError, NetworkError};
use std::sync::Arc;
use tokio::{io::AsyncWriteExt, net::TcpStream, sync::Mutex};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Client {
    id: Uuid,
    stream: Arc<Mutex<TcpStream>>,
    is_connected: Arc<Mutex<bool>>,
}

impl Client {
    pub fn new(stream: Arc<Mutex<TcpStream>>) -> Self {
        Self {
            id: Uuid::new_v4(),
            stream,
            is_connected: Arc::new(Mutex::new(true)),
        }
    }

    pub async fn handle_message(&mut self) -> Result<(), ClientError> {
        // TODO: read message from client
        Ok(())
    }

    pub async fn run_pipeline(&mut self) -> Result<(), ClientError> {
        println!("Running client pipeline for {:?}", self.id);
        // TODO: send message to client
        // TODO: read message from client
        loop {
            if !self.is_connected().await {
                break;
            }
            match self.handle_message().await {
                Ok(_) => {}
                Err(e) => {
                    println!("Error handling message for {:?}: {:?}", self.id, e);
                    break;
                }
            }
        }
        let result = self.stop().await;
        match result {
            Ok(_) => {}
            Err(e) => {
                println!("Error stopping client for {:?}: {:?}", self.id, e);
            }
        }
        println!("Client pipeline for {:?} finished", self.id);
        Ok(())
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub async fn is_connected(&self) -> bool {
        *self.is_connected.lock().await
    }

    pub async fn stop(&mut self) -> Result<(), NetworkError> {
        let mut stream = self.stream.lock().await;
        stream
            .shutdown()
            .await
            .map_err(|e| NetworkError::ShutdownError(e.to_string()))?;
        *self.is_connected.lock().await = false;
        Ok(())
    }
}
