pub mod error;
pub mod network;
pub mod pipeline;

pub use error::{BotError, ClientError, NetworkError};

pub use network::client::Client;
pub use network::manager::NetworkManager;
