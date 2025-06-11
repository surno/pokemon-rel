use pokebot_rust::network::frame_handler::PokemonFrameHandler;
use pokebot_rust::pipeline::services::FanoutService;
use pokebot_rust::{NetworkManager, app::VisualizationApp};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create fan-out service with visualization
    let (fanout_service, viz_receiver) = FanoutService::new(10);

    // Start GUI in separate thread
    std::thread::spawn(move || {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            VisualizationApp::start_gui(viz_receiver).await;
        });
    });

    let pokemon_handler = PokemonFrameHandler::new(fanout_service);

    // Start network manager with fan-out handler
    let (mut manager, _) = NetworkManager::new(3344);

    println!("🚀 Pokemon Bot started with visualization!");
    println!("📺 Debug window should open automatically");

    manager.start().await?;
    Ok(())
}
