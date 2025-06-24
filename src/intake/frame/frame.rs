use image::DynamicImage;
use uuid::Uuid;

use crate::{error::AppError, intake::frame::visitor::FrameVisitor};

pub enum Frame {
    Ping,
    Handshake { id: Uuid, program: u16 },
    Image { image: DynamicImage },
    Shutdown,
}

impl Frame {
    pub fn accept<V: FrameVisitor + Send + Sync + ?Sized>(
        &self,
        visitor: &mut V,
    ) -> Result<(), AppError> {
        match self {
            Frame::Ping => visitor.ping(),
            Frame::Handshake { id, program } => visitor.handshake(*id, *program),
            Frame::Image { image } => visitor.image(image.clone()),
            Frame::Shutdown => visitor.shutdown(),
        }
    }
}
