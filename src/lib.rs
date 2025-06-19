pub mod app;
pub mod error;
pub mod intake;
pub mod network;
pub mod pipeline;

pub use app::multiclient_app::MultiClientApp;

pub use intake::client::{Client, ClientHandle, ClientManager};
pub use intake::frame::Frame;
pub use intake::frame::frame_handler::PokemonFrameHandler;
pub use network::manager::NetworkManager;
