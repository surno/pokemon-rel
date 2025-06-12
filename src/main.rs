use std::sync::Arc;

use pokebot_rust::{
    NetworkManager, app::multiclient_app::MultiClientApp,
    network::client::client_manager::ClientManager,
};
use tracing::Level;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // enable debug logging
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();

    // Shared client manager using tokio::sync::RwLock
    let client_manager = Arc::new(tokio::sync::RwLock::new(ClientManager::new()));

    // Start network manager
    let network_client_manager = client_manager.clone();
    tokio::spawn(async move {
        let (mut manager, _) = NetworkManager::new(3344, network_client_manager);
        manager.start().await.unwrap();
    });

    // Start GUI using the same shared client manager
    let _app = MultiClientApp::new(client_manager);
    MultiClientApp::start_gui();

    Ok(())
}
