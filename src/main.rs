use pokebot_rust::network::frame_handler::PokemonFrameHandler;
use pokebot_rust::pipeline::services::FanoutService;
use pokebot_rust::{NetworkManager, app::multiclient_app::MultiClientApp};
use tracing::{Level, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // enable debug logging
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();

    // Start network manager with fan-out handler on a separate thread
    tokio::spawn(async move {
        let (mut manager, _) = NetworkManager::new(3344);
        manager.start().await.unwrap();
    });

    MultiClientApp::start_gui().await;

    Ok(())
}
