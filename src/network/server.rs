use crate::intake::client::client_manager::FrameReaderClientManager;
use crate::pipeline::services::MLPipelineService;
use crate::pipeline::services::preprocessing::SceneAnnotationService;
use crate::pipeline::{EnrichedFrame, FramePublishingService, Scene};
use crate::{error::AppError, intake::client::ClientHandle};
use bloomfilter::Bloom;
use core::net::SocketAddr;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::RwLock,
};

use tower::{Service, ServiceBuilder, ServiceExt};
use tracing::{debug, error, info};

pub struct Server {
    client_handles: Arc<RwLock<Vec<ClientHandle>>>,
    bloom_filters: Arc<HashMap<Scene, Bloom<String>>>,
    port: u16,
}

impl Server {
    pub fn new(port: u16, _: Arc<RwLock<FrameReaderClientManager>>) -> Self {
        let client_handles = Arc::new(RwLock::new(Vec::new()));
        let bloom_filters = Arc::new(HashMap::new());
        Self {
            client_handles: client_handles.clone(),
            bloom_filters,
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
        let (frame_publishing_service, _) = FramePublishingService::new();

        let mut scene_annotation_service = SceneAnnotationService::new(self.bloom_filters.clone());

        // let mut client_service = ServiceBuilder::new()
        //     .concurrency_limit(10)
        //     .timeout(Duration::from_secs(30))
        //     .service(
        //         tower::service_fn(|frame: EnrichedFrame| async move {
        //             scene_annotation_service.call(frame).await.map_err(|e| {
        //                 error!("Error annotating scene: {:?}", e);
        //                 e
        //             })
        //         })
        //         .and_then(frame_publishing_service),
        //     );

        tokio::spawn(async move { todo!() });
        Ok(())
    }
}
