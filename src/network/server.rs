use crate::intake::client::client_manager::FrameReaderClientManager;
use crate::pipeline::services::MLPipelineService;
use crate::{error::AppError, intake::client::ClientHandle};
use core::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::RwLock,
};

use tower::ServiceBuilder;
use tracing::{debug, error, info};

pub struct Server {
    client_handles: Arc<RwLock<Vec<ClientHandle>>>,
    port: u16,
}

impl Server {
    pub fn new(port: u16, _: Arc<RwLock<FrameReaderClientManager>>) -> Self {
        let client_handles = Arc::new(RwLock::new(Vec::new()));
        Self {
            client_handles: client_handles.clone(),
            port,
        }
    }

    pub async fn start(&mut self) -> Result<(), AppError> {
        info!("Starting network manager on port {}", self.port);
        let listener = TcpListener::bind(format!("0.0.0.0:{}", self.port))
            .await
            .map_err(|e| AppError::Bind(e, self.port))?;
        loop {
            let result = listener.accept().await;
            if let Ok((stream, peer)) = result {
                let result = self.handle_client(stream, peer).await;
                if let Err(e) = result {
                    error!("Error constructing client: {:?} from {:?}", e, peer);
                }
            } else {
                error!(
                    "Error accepting connection: {:?} from unknown peer",
                    result.err()
                );
            }
        }
    }

    async fn handle_client(&self, _: TcpStream, peer: SocketAddr) -> Result<(), AppError> {
        debug!("New client attempting to connect: {:?}", peer);
        let mut _service = ServiceBuilder::new()
            .concurrency_limit(10)
            .timeout(Duration::from_secs(30))
            .service(MLPipelineService::new());

        tokio::spawn(async move { todo!() });
        Ok(())
    }
}
