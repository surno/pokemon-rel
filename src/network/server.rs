use crate::error::AppError;
use tokio::net::{TcpListener, TcpStream};

use tracing::{debug, error, info};

pub struct Server {
    port: u16,
}

impl Server {
    pub fn new(port: u16) -> Self {
        Self { port }
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
                let result = self.handle_client(stream).await;
                if let Err(e) = result {
                    error!("Error handling client: {:?} from {:?}", e, peer);
                }
            } else {
                error!(
                    "Error accepting connection: {:?} from unknown peer",
                    result.err()
                );
            }
        }
    }

    async fn handle_client(&self, _: TcpStream) -> Result<(), AppError> {
        // tokio::spawn(async move {
        //     let mut reader = FramedAsyncBufferedReader::new(stream);
        //     loop {
        //         let frame: Result<crate::Frame, crate::error::FrameError> = reader.read().await;
        //         match frame {
        //             Ok(frame) => {
        //                 self.publisher.send(Arc::new(frame));
        //             }
        //             Err(e) => {
        //                 error!("Error reading frame: {:?}", e);
        //             }
        //         }
        //     }
        // });
        Ok(())
    }
}
