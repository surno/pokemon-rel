use crate::error::FrameError;

#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    Ping,
    Handshake {
        version: u32,
        name: String,
        program: u16,
    },
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
                let version =
                    u32::from_le_bytes(slice[1..5].try_into().map_err(FrameError::InvalidVersion)?);
                let name_length = u16::from_le_bytes(
                    slice[5..7]
                        .try_into()
                        .map_err(FrameError::InvalidNameLength)?,
                );
                let name = String::from_utf8(slice[7..7 + name_length as usize].to_vec())
                    .map_err(FrameError::InvalidName)?;
                let program = u16::from_le_bytes(
                    slice[7 + name_length as usize..7 + name_length as usize + 2] // TODO: check if this is correct
                        .try_into()
                        .map_err(FrameError::InvalidProgram)?,
                );
                Ok(Frame::Handshake {
                    version,
                    name,
                    program,
                })
            }
            2 => {
                let width =
                    u32::from_le_bytes(slice[1..5].try_into().map_err(FrameError::InvalidWidth)?);
                let height =
                    u32::from_le_bytes(slice[5..9].try_into().map_err(FrameError::InvalidHeight)?);
                let pixels = slice[9..].to_vec();
                // verify the pixels match the width and height
                if pixels.len() != (width * height) as usize {
                    return Err(FrameError::InvalidPixelsLength(
                        width,
                        height,
                        (width * height) as usize,
                        pixels.len(),
                    ));
                }
                Ok(Frame::Image {
                    width,
                    height,
                    pixels,
                })
            }
            3 => Ok(Frame::Shutdown),
            _ => Err(FrameError::InvalidFrameTag(tag)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_try_from_ping() {
        let data: [u8; 10] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let frame = Frame::try_from(&data[..]).unwrap();
        assert_eq!(frame, Frame::Ping);
    }
    #[test]
    fn test_frame_try_from_handshake() {
        let data: [u8; 14] = [1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let frame = Frame::try_from(&data[..]);
        if let Ok(frame) = frame {
            assert_eq!(
                frame,
                Frame::Handshake {
                    version: 1,
                    name: String::new(),
                    program: 0,
                }
            );
        } else {
            panic!("Error: {:?}", frame.unwrap_err().to_string());
        }
    }

    #[test]
    fn test_frame_try_from_image() {
        let data: [u8; 9] = [2, 0, 0, 0, 0, 0, 0, 0, 0];
        let frame = Frame::try_from(&data[..]).unwrap();
        assert_eq!(
            frame,
            Frame::Image {
                width: 0,
                height: 0,
                pixels: vec![],
            }
        );
    }

    #[test]
    fn test_frame_try_from_shutdown() {
        let data: [u8; 10] = [3, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let frame = Frame::try_from(&data[..]).unwrap();
        assert_eq!(frame, Frame::Shutdown);
    }

    #[test]
    fn test_frame_try_from_invalid_tag() {
        let data: [u8; 10] = [4, 0, 0, 0, 0, 0, 0, 0, 0, 0];
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
    fn test_frame_try_from_invalid_pixels() {
        let data: [u8; 10] = [2, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let frame = Frame::try_from(&data[..]);
        assert!(frame.is_err());
    }
}
