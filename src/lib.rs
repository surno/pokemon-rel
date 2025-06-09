pub mod error;
pub mod network;

pub use error::{BotError, ClientError, NetworkError};

pub use network::client::Client;
pub use network::manager::NetworkManager;
