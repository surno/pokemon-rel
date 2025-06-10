use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct RawFrame {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
    pub timestamp: u64,
    pub id: Uuid,
}
