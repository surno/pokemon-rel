use crate::{
    error::FrameError,
    intake::frame::{
        Frame,
        reader::{FrameReader, frame_reader::ReadState},
    },
};
use std::future::Future;
use std::pin::Pin;
use tokio::io::{AsyncReadExt, BufReader, Interest};
use tokio::net::TcpStream;

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
        let mut length_buffer = [0u8; 4];
        let bytes_read: usize = self
            .reader
            .read_exact(&mut length_buffer)
            .await
            .map_err(FrameError::Read)?;

        if bytes_read != 4 {
            return Err(FrameError::InvalidFrameLength(bytes_read));
        }

        Ok(u32::from_le_bytes(length_buffer))
    }

    async fn read_frame_data(&mut self, expected_length: u32) -> Result<Frame, FrameError> {
        let mut frame_buffer = vec![0u8; expected_length as usize];
        self.reader
            .read_exact(&mut frame_buffer)
            .await
            .map_err(FrameError::Read)?;

        Ok(Frame::try_from(frame_buffer.as_slice())?)
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

    fn is_connected<'a>(&'a self) -> Pin<Box<dyn Future<Output = bool> + Send + 'a>> {
        Box::pin(async move {
            self.reader
                .get_ref()
                .ready(Interest::READABLE)
                .await
                .is_ok()
        })
    }
}
