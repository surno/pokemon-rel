use crate::{
    error::FrameError,
    intake::frame::{
        Frame,
        reader::{FrameReader, frame_reader::ReadState},
    },
};
use bytes::BytesMut;
use std::future::Future;
use std::pin::Pin;
use tokio::io::{AsyncReadExt, BufReader};
use tokio::net::TcpStream;

const FRAME_LENGTH_BYTES: usize = 4;

pub struct FramedTcpReader {
    reader: BufReader<TcpStream>,
}

impl FramedTcpReader {
    pub fn new(stream: TcpStream) -> Self {
        Self {
            reader: BufReader::new(stream),
        }
    }

    pub async fn read_frame_length(&mut self) -> Result<u32, FrameError> {
        // [length][tag][data]
        // [length] is 4 bytes
        let mut length_buffer = [0u8; FRAME_LENGTH_BYTES];
        let bytes_read: usize = self
            .reader
            .read_exact(&mut length_buffer)
            .await
            .map_err(FrameError::Read)?;

        if bytes_read != FRAME_LENGTH_BYTES {
            return Err(FrameError::InvalidFrameLength(
                FRAME_LENGTH_BYTES,
                bytes_read,
            ));
        }

        Ok(u32::from_le_bytes(length_buffer))
    }

    async fn read_frame_data(&mut self, expected_length: u32) -> Result<Frame, FrameError> {
        let mut bytes = BytesMut::with_capacity(expected_length as usize);
        let bytes_read = self
            .reader
            .read_exact(&mut bytes)
            .await
            .map_err(FrameError::Read)?;

        if bytes_read != expected_length as usize {
            return Err(FrameError::InvalidFrameLength(
                expected_length as usize,
                bytes_read,
            ));
        }

        Ok(Frame::try_from(bytes.freeze())?)
    }
}

impl FrameReader for FramedTcpReader {
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
