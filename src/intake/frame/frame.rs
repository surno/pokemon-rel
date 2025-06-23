use image::DynamicImage;

use crate::{error::AppError, intake::frame::visitor::FrameVisitor};

pub enum Frame {
    Ping,
    Handshake {
        version: u32,
        name: String,
        program: u16,
    },
    Image {
        image: DynamicImage,
    },
    Shutdown,
}

impl Frame {
    pub fn accept<V: FrameVisitor + Send + Sync + ?Sized>(
        &self,
        visitor: &mut V,
    ) -> Result<(), AppError> {
        match self {
            Frame::Ping => visitor.ping(),
            Frame::Handshake {
                version,
                name,
                program,
            } => visitor.handshake(*version, name.clone(), *program),
            Frame::Image { image } => visitor.image(image.clone()),
            Frame::Shutdown => visitor.shutdown(),
        }
    }
}
