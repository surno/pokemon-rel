pub mod error;
pub mod network;

pub use error::{BotError, NetworkError};

pub use network::client::Client;
pub use network::manager::NetworkManager;
