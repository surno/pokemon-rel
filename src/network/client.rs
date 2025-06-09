use std::sync::Arc;
use tokio::net::TcpStream;

#[derive(Debug, Clone)]
pub struct Client {
    pub stream: Arc<TcpStream>,
    pub is_connected: bool,
}

impl Client {
    pub fn new(stream: Arc<TcpStream>) -> Self {
        Self {
            stream,
            is_connected: false,
        }
    }
}
