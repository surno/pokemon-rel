use crate::{
    error::AppError,
    intake::{
        client::manager::ClientManagerHandle,
        frame::{reader::FramedAsyncBufferedReader, writer::FramedAsyncBufferedWriter},
    },
};
use tokio::net::{TcpListener, TcpStream};

use tracing::{debug, error, info};
use uuid::Uuid;

pub struct Server {
    port: u16,
    client_manager: ClientManagerHandle,
}

impl Server {
    pub fn new(port: u16, client_manager: ClientManagerHandle) -> Self {
        Self {
            port,
            client_manager,
        }
    }

    pub async fn start(&mut self) -> Result<(), AppError> {
        info!("Starting network server on port {}", self.port);
        let listener = TcpListener::bind(format!("0.0.0.0:{}", self.port))
            .await
            .map_err(|e| AppError::Bind(e, self.port))?;
        loop {
            let result = listener.accept().await;
            if let Ok((stream, peer)) = result {
                debug!("New client attempting to connect: {:?}", peer);
                let client_id = self.handle_client(stream).await?;
                debug!("Client connected: {:?} for peer {:?}", client_id, peer);
            } else {
                error!(
                    "Error accepting connection: {:?} from unknown peer",
                    result.err()
                );
            }
        }
    }

    async fn handle_client(&self, stream: TcpStream) -> Result<Uuid, AppError> {
        let client_manager = &self.client_manager;
        let (stream_rx, stream_tx) = stream.into_split();
        let reader = FramedAsyncBufferedReader::new(stream_rx);
        let writer = FramedAsyncBufferedWriter::new(stream_tx);
        client_manager
            .add_client(Box::new(reader), Box::new(writer))
            .await
    }
}
