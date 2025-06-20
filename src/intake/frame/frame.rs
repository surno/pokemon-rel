use bytes::Bytes;
use tracing::error;

use crate::error::FrameError;

#[derive(PartialEq)]
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
    ImageGD2 {
        width: u32,
        height: u32,
        gd2_data: Vec<u8>,
    },
    Shutdown,
}

impl TryFrom<Bytes> for Frame {
    type Error = FrameError;

    fn try_from(slice: Bytes) -> Result<Self, Self::Error> {
        if slice.len() < 5 {
            return Err(FrameError::InvalidFrameLength(5, slice.len()));
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
                // verify the pixels match the width and height for RGB format (3 bytes per pixel)
                let expected_rgb_size = (width * height * 3) as usize;
                if pixels.len() != expected_rgb_size {
                    error!(
                        "Invalid pixels length, got {}x{}*3 != {}, expected {}",
                        width,
                        height,
                        pixels.len(),
                        expected_rgb_size
                    );
                    return Err(FrameError::InvalidPixelsLength(
                        width,
                        height,
                        expected_rgb_size,
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
            4 => {
                // GD2 image format - no strict size validation since GD2 is compressed
                let width =
                    u32::from_le_bytes(slice[1..5].try_into().map_err(FrameError::InvalidWidth)?);
                let height =
                    u32::from_le_bytes(slice[5..9].try_into().map_err(FrameError::InvalidHeight)?);
                let gd2_data = slice[9..].to_vec();

                // Basic validation - GD2 files should start with "GD2"
                if gd2_data.len() < 4 {
                    return Err(FrameError::InvalidPixelsLength(
                        width,
                        height,
                        4, // minimum for GD2 header
                        gd2_data.len(),
                    ));
                }

                Ok(Frame::ImageGD2 {
                    width,
                    height,
                    gd2_data,
                })
            }
            _ => Err(FrameError::InvalidFrameTag(tag)),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::vec;

    use bytes::BytesMut;

    use super::*;

    #[test]
    fn test_frame_try_from_ping() {
        let data: Bytes = Bytes::from_static(&[0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        let frame = Frame::try_from(data).unwrap();
        assert!(matches!(frame, Frame::Ping));
    }
    #[test]
    fn test_frame_try_from_handshake() {
        let data: Bytes = Bytes::from_static(&[1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        let frame = Frame::try_from(data);
        match frame {
            Ok(frame) => {
                if let Frame::Handshake {
                    version,
                    name,
                    program,
                } = frame
                {
                    assert_eq!(version, 1);
                    assert_eq!(name, String::new());
                    assert_eq!(program, 0);
                } else {
                    panic!("Expected Handshake frame");
                }
            }
            Err(e) => {
                panic!("Error: {:?}", e);
            }
        }
    }

    #[test]
    fn test_frame_try_from_image() {
        let data: Bytes = Bytes::from_static(&[2, 0, 0, 0, 0, 0, 0, 0, 0]);
        let frame = Frame::try_from(data).unwrap();
        let _expected_pixels: Vec<u8> = vec![];
        assert!(matches!(
            frame,
            Frame::Image {
                width: 0,
                height: 0,
                pixels: _expected_pixels,
            }
        ));
    }

    #[test]
    fn test_frame_try_from_shutdown() {
        let data: Bytes = Bytes::from_static(&[3, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        let frame = Frame::try_from(data).unwrap();
        assert!(matches!(frame, Frame::Shutdown));
    }

    #[test]
    fn test_frame_try_from_invalid_tag() {
        let data: Bytes = Bytes::from_static(&[4, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        let frame = Frame::try_from(data);
        assert!(frame.is_err());
    }

    #[test]
    fn test_frame_try_from_invalid_length() {
        let data: Bytes = Bytes::from_static(&[1, 0, 0, 0]);
        let frame = Frame::try_from(data);
        assert!(frame.is_err());
    }

    #[test]
    fn test_frame_try_from_invalid_pixels() {
        // Test case: 1x1 image should need 3 bytes (RGB), but we only provide 1 byte
        let data: Bytes = Bytes::from_static(&[2, 1, 0, 0, 0, 1, 0, 0, 0, 255]); // 1x1 image with 1 byte (should need 3)
        let frame = Frame::try_from(data);
        assert!(frame.is_err());
    }

    #[test]
    fn test_frame_try_from_valid_rgb_pixels() {
        // Test case: 1x1 RGB image with correct 3 bytes
        let data: Bytes = Bytes::from_static(&[2, 1, 0, 0, 0, 1, 0, 0, 0, 255, 128, 64]); // 1x1 image with RGB
        let frame = Frame::try_from(data).unwrap();
        let _expected_pixels: Vec<u8> = vec![255, 128, 64];
        assert!(matches!(
            frame,
            Frame::Image {
                width: 1,
                height: 1,
                pixels: _expected_pixels,
            }
        ));
    }

    #[test]
    fn test_frame_try_from_gd2_image() {
        // Test case: GD2 format with mock GD2 header
        let mut data = BytesMut::new(); // tag=4, width=1, height=1
        data.extend_from_slice(&[4, 1, 0, 0, 0, 1, 0, 0, 0]);
        data.extend_from_slice(b"GD2\001mock_gd2_data"); // Mock GD2 data
        let frame = Frame::try_from(data.freeze()).unwrap();
        let _expected_gd2_data: Vec<u8> = b"GD2\001mock_gd2_data".to_vec();
        assert!(matches!(
            frame,
            Frame::ImageGD2 {
                width: 1,
                height: 1,
                gd2_data: _expected_gd2_data,
            }
        ));
    }
}
