use pokebot_rust::network::frame_handler::PokemonFrameHandler;
use pokebot_rust::pipeline::services::FanoutService;
use pokebot_rust::{NetworkManager, app::VisualizationApp};
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    // Create fan-out service with visualization
    let (fanout_service, viz_receiver) = FanoutService::new(10);

    info!("Starting fan-out service");

    // Start network manager with fan-out handler on a separate thread
    tokio::spawn(async move {
        info!("Starting network manager on port 3344");
        let pokemon_handler = PokemonFrameHandler::new(fanout_service);
        let (mut manager, _) = NetworkManager::new(3344);
        info!("Network manager started on port 3344");
        manager.start().await.unwrap();
    });

    VisualizationApp::start_gui(viz_receiver).await;

    Ok(())
}
