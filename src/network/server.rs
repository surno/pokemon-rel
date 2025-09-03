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
        Server {
            port,
            client_manager,
        }
    }

    pub async fn start(&mut self) -> Result<(), AppError> {
        let addr = format!("127.0.0.1:{}", self.port);
        let listener = TcpListener::bind(&addr).await.map_err(AppError::Io)?;
        info!("Server listening on {}", addr);

        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    info!("New client connected");
                    let (reader, writer) = stream.into_split();
                    let client_manager = self.client_manager.clone();
                    tokio::spawn(async move {
                        let reader = FramedAsyncBufferedReader::new(reader);
                        let writer = FramedAsyncBufferedWriter::new(writer);
                        client_manager
                            .add_client(Box::new(reader), Box::new(writer))
                            .await
                            .unwrap();
                    });
                }
                Err(e) => {
                    error!("Failed to accept connection: {}", e);
                }
            }
        }
    }
}
