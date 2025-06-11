use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct RawFrame {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
    pub timestamp: u64,
    pub id: Uuid,
}

impl RawFrame {
    pub fn new(width: u32, height: u32, pixels: Vec<u8>) -> Self {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let id = Uuid::new_v4();
        Self {
            width,
            height,
            pixels,
            timestamp,
            id,
        }
    }
}
