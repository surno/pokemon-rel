use crate::error::FrameError;

#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    Ping,
    Image {
        width: u32,
        height: u32,
        pixels: Vec<u8>,
    },
    Shutdown,
}

impl TryFrom<&[u8]> for Frame {
    type Error = FrameError;

    fn try_from(slice: &[u8]) -> Result<Self, Self::Error> {
        if slice.len() < 5 {
            return Err(FrameError::InvalidFrameLength(slice.len()));
        }
        let tag = slice[0];
        match tag {
            0 => Ok(Frame::Ping),
            1 => {
                let width = u32::from_le_bytes(slice[1..5].try_into().unwrap());
                let height = u32::from_le_bytes(slice[5..9].try_into().unwrap());
                let pixels = slice[9..].to_vec();
                // verify the pixels match the width and height
                if pixels.len() != (width * height) as usize {
                    return Err(FrameError::InvalidFrameLength(slice.len()));
                }
                Ok(Frame::Image {
                    width,
                    height,
                    pixels,
                })
            }
            2 => Ok(Frame::Shutdown),
            _ => Err(FrameError::InvalidFrameTag(tag)),
        }
    }
}

pub trait FrameHandler: Send + Sync + 'static {
    fn handle_ping(&self) -> Result<(), FrameError>;
    fn handle_image(&self, width: u32, height: u32, pixels: Vec<u8>) -> Result<(), FrameError>;
    fn handle_shutdown(&self) -> Result<(), FrameError>;
}

pub struct DelegatingRouter<H: FrameHandler> {
    handler: H,
}

impl<H: FrameHandler> DelegatingRouter<H> {
    pub async fn route(&self, data: &[u8]) -> Result<(), FrameError> {
        let frame = Frame::try_from(data)?;
        match frame {
            Frame::Ping => self.handler.handle_ping(),
            Frame::Image {
                width,
                height,
                pixels,
            } => self.handler.handle_image(width, height, pixels),
            Frame::Shutdown => self.handler.handle_shutdown(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_try_from() {
        let data: [u8; 10] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let frame = Frame::try_from(&data[..]).unwrap();
        assert_eq!(frame, Frame::Ping);

        let data: [u8; 9] = [1, 0, 0, 0, 0, 0, 0, 0, 0];
        let frame = Frame::try_from(&data[..]).unwrap();
        assert_eq!(
            frame,
            Frame::Image {
                width: 0,
                height: 0,
                pixels: vec![],
            }
        );

        let data: [u8; 10] = [2, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let frame = Frame::try_from(&data[..]).unwrap();
        assert_eq!(frame, Frame::Shutdown);

        let data: [u8; 10] = [3, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let frame = Frame::try_from(&data[..]);
        assert!(frame.is_err());
    }

    #[test]
    fn test_frame_try_from_invalid_length() {
        let data: [u8; 4] = [1, 0, 0, 0];
        let frame = Frame::try_from(&data[..]);
        assert!(frame.is_err());
    }

    #[test]
    fn test_frame_try_from_invalid_tag() {
        let data: [u8; 5] = [3, 0, 0, 0, 0];
        let frame = Frame::try_from(&data[..]);
        assert!(frame.is_err());
    }

    #[test]
    fn test_frame_try_from_invalid_pixels() {
        let data: [u8; 10] = [1, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let frame = Frame::try_from(&data[..]);
        assert!(frame.is_err());
    }
}
