use thiserror::Error;

// Main Applicaiton Error Type

#[derive(Error, Debug)]
pub enum BotError {
    #[error("Network Error: {0}")]
    NetworkError(#[from] NetworkError),
}

// Network Error Type
#[derive(Error, Debug)]
pub enum NetworkError {
    #[error("Failed to bind to port {1}: {0}")]
    BindError(std::io::Error, u16),
    #[error("Failed to accept connection: {0}")]
    AcceptError(std::io::Error),
    #[error("Failed to shutdown connection: {0}")]
    ShutdownError(std::io::Error),
}
