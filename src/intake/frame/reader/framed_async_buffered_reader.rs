use crate::{
    error::FrameError,
    intake::frame::{Frame, reader::FrameReader},
};
use image::{DynamicImage, RgbImage};
use std::pin::Pin;
use tokio::io::{AsyncReadExt, BufReader};
use uuid::Uuid;

const FRAME_LENGTH_BYTES: usize = 4;

pub enum ReadState {
    WaitingForLength,
    WaitingForFrame { expected_length: u32 },
}

pub struct FramedAsyncBufferedReader<T>
where
    T: AsyncReadExt + Unpin + Sync + Send,
{
    reader: BufReader<T>,
}

impl<T: AsyncReadExt + Unpin + Sync + Send> FramedAsyncBufferedReader<T> {
    pub fn new(stream: T) -> Self {
        Self {
            reader: BufReader::new(stream),
        }
    }

    fn read_frame_length<'a>(
        &'a mut self,
    ) -> Pin<Box<dyn Future<Output = Result<u32, FrameError>> + Send + 'a>> {
        Box::pin(async move {
            // [length][tag][data]
            // [length] is 4 bytes
            let mut length_buffer = [0u8; FRAME_LENGTH_BYTES];
            let future_read = self.reader.read_exact(&mut length_buffer);
            let bytes_read = future_read.await.map_err(FrameError::Read)?;
            if bytes_read != FRAME_LENGTH_BYTES {
                return Err(FrameError::InvalidFrameLength(
                    FRAME_LENGTH_BYTES,
                    bytes_read,
                ));
            }
            Ok(u32::from_le_bytes(length_buffer))
        })
    }

    fn read_frame_data<'a>(
        &'a mut self,
        expected_length: u32,
    ) -> Pin<Box<dyn Future<Output = Result<Frame, FrameError>> + Send + 'a>> {
        Box::pin(async move {
            let mut total_bytes_read = 0;
            let mut tag_buffer = [0u8; 1];
            total_bytes_read += self
                .reader
                .read_exact(&mut tag_buffer)
                .await
                .map_err(FrameError::Read)?;
            let frame_return: Option<Frame>;
            let tag = tag_buffer[0];
            match tag {
                0 => {
                    frame_return = Some(Frame::Ping);
                }
                1 => {
                    let (frame, bytes_read) = read_handshake(&mut self.reader).await?;
                    total_bytes_read += bytes_read;
                    frame_return = Some(frame);
                }
                2 => {
                    let (frame, bytes_read) = read_rgb_image(&mut self.reader).await?;
                    total_bytes_read += bytes_read;
                    frame_return = Some(frame);
                }
                3 => {
                    // Shutdown frame
                    frame_return = Some(Frame::Shutdown);
                }
                _ => {
                    return Err(FrameError::InvalidFrameLength(
                        expected_length as usize,
                        total_bytes_read,
                    ));
                }
            }

            if total_bytes_read != expected_length as usize {
                return Err(FrameError::InvalidFrameLength(
                    expected_length as usize,
                    total_bytes_read,
                ));
            }
            match frame_return {
                Some(frame) => Ok(frame),
                None => Err(FrameError::InvalidFrameLength(
                    expected_length as usize,
                    total_bytes_read,
                )),
            }
        })
    }
}

impl<T: AsyncReadExt + Unpin + Sync + Send> FrameReader for FramedAsyncBufferedReader<T> {
    fn read<'a>(
        &'a mut self,
    ) -> Pin<Box<dyn Future<Output = Result<Frame, FrameError>> + Send + 'a>> {
        Box::pin(async move {
            let mut state = ReadState::WaitingForLength;
            loop {
                match &mut state {
                    ReadState::WaitingForLength => {
                        state = ReadState::WaitingForFrame {
                            expected_length: self.read_frame_length().await?,
                        };
                    }
                    ReadState::WaitingForFrame { expected_length } => {
                        return self.read_frame_data(*expected_length).await;
                    }
                }
            }
        })
    }
}

async fn read_rgb_image<T>(buf_reader: &mut BufReader<T>) -> Result<(Frame, usize), FrameError>
where
    T: AsyncReadExt + Unpin,
{
    let mut bytes_read = 0;
    let mut width_buffer = [0u8; 4];
    bytes_read += buf_reader
        .read_exact(&mut width_buffer)
        .await
        .map_err(FrameError::Read)?;
    let width = u32::from_le_bytes(width_buffer);
    let mut height_buffer = [0u8; 4];
    bytes_read += buf_reader
        .read_exact(&mut height_buffer)
        .await
        .map_err(FrameError::Read)?;
    let height = u32::from_le_bytes(height_buffer);
    let mut pixels_buffer = vec![0u8; (width * height * 3) as usize];
    bytes_read += buf_reader
        .read_exact(&mut pixels_buffer)
        .await
        .map_err(FrameError::Read)?;
    Ok((
        Frame::Image {
            image: DynamicImage::ImageRgb8(
                RgbImage::from_raw(width, height, pixels_buffer).unwrap(),
            ),
        },
        bytes_read,
    ))
}

async fn read_handshake<T>(buf_reader: &mut BufReader<T>) -> Result<(Frame, usize), FrameError>
where
    T: AsyncReadExt + Unpin,
{
    let mut bytes_read = 0;
    let mut program_buffer = [0u8; 2];
    bytes_read += buf_reader
        .read_exact(&mut program_buffer)
        .await
        .map_err(FrameError::Read)?;
    Ok((
        Frame::Handshake {
            id: Uuid::new_v4(),
            program: u16::from_le_bytes(program_buffer),
        },
        bytes_read,
    ))
}
