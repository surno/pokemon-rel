use thiserror::Error;
use uuid::Uuid;

// Main Applicaiton Error Type

#[derive(Error, Debug)]
pub enum BotError {
    #[error("Network Error: {0}")]
    NetworkError(#[from] NetworkError),
    #[error("Client Error: {0}")]
    ClientError(#[from] ClientError),
}

// Network Error Type
#[derive(Error, Debug)]
pub enum NetworkError {
    #[error("Failed to bind to port {1}: {0}")]
    BindError(std::io::Error, u16),
    #[error("Failed to accept connection: {0}")]
    AcceptError(std::io::Error),
    #[error("Failed to shutdown the server: {0}")]
    ShutdownError(String),
    #[error("The server is already started.")]
    AlreadyStarted,
}

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("Failed to read message: {0}")]
    ReadError(std::io::Error),
    #[error("Failed to write message: {0}")]
    WriteError(std::io::Error),
    #[error("Failed to send shutdown to client handle: {0}")]
    ShutdownError(Uuid),
    #[error("Failed to stop client: {0}")]
    StopError(NetworkError),
}
