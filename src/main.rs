use std::sync::Arc;

use pokebot_rust::{
    NetworkManager, app::multiclient_app::MultiClientApp,
    network::client::client_manager::ClientManager,
};
use tokio::sync::RwLock;
use tracing::Level;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // enable debug logging
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();

    let client_manager = Arc::new(RwLock::new(ClientManager::new()));

    // Start network manager with fan-out handler on a separate thread
    tokio::spawn(async move {
        let (mut manager, _) = NetworkManager::new(3344, client_manager);
        manager.start().await.unwrap();
    });

    MultiClientApp::start_gui().await;

    Ok(())
}
