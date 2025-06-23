use std::sync::Arc;

use pokebot_rust::{
    Server, app::multiclient_app::MultiClientApp, intake::client::manager::ClientManager,
};
use tracing::Level;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // enable debug logging
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    // Shared client manager using tokio::sync::RwLock
    let client_manager = ClientManager::new();

    // Start tokio runtime and network manager in a separate thread
    let network_client_manager = client_manager.clone();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mut server = Server::new(3344);
            server.start().await.unwrap();
        });
    });

    // Run GUI on the main thread (required by macOS)
    MultiClientApp::start_gui(client_manager);

    Ok(())
}
