use pokebot_rust::app::multiclient_app::MultiClientApp;
use tracing::Level;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // enable debug logging
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();

    // Run GUI on the main thread (required by macOS)
    MultiClientApp::start_gui();

    Ok(())
}
