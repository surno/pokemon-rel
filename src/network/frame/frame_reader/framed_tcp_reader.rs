use crate::{
    error::FrameError,
    network::frame::{
        Frame,
        iframe_reader::{IFrameReader, ReadState},
    },
};
use tokio::io::{AsyncReadExt, BufReader, Interest};
use tokio::net::TcpStream;
use tracing::debug;

#[derive(Debug)]
pub struct FramedTcpReader {
    reader: BufReader<TcpStream>,
    state: ReadState,
    buffer: Vec<u8>,
}

impl FramedTcpReader {
    pub fn new(stream: TcpStream) -> Self {
        Self {
            reader: BufReader::new(stream),
            state: ReadState::WaitingForLength,
            buffer: vec![],
        }
    }
}

impl IFrameReader for FramedTcpReader {
    fn read(&mut self) -> impl std::future::Future<Output = Result<Frame, FrameError>> + Send {
        async move {
            loop {
                match &self.state {
                    ReadState::WaitingForLength => {
                        // [length][tag][data]
                        // [length] is 4 bytes
                        while self.buffer.len() < 4 {
                            let mut temp_buffer = [0u8; 1024];
                            let bytes_read = self
                                .reader
                                .read(&mut temp_buffer)
                                .await
                                .map_err(FrameError::Read)?;

                            if bytes_read == 0 {
                                debug!("Connection closed while reading frame length");
                                return Err(FrameError::InvalidFrameLength(0)); // Connection is closed
                            }

                            self.buffer.extend_from_slice(&temp_buffer[..bytes_read]);
                        }

                        let length_bytes: [u8; 4] = self.buffer[..4]
                            .try_into()
                            .map_err(|_| FrameError::InvalidFrameLength(self.buffer.len()))?;

                        let frame_length = u32::from_le_bytes(length_bytes);

                        debug!("Read frame length: {}", frame_length);

                        // Remove the length from the buffer, downstream doesn't need it anymore
                        self.buffer.drain(..4);

                        self.state = ReadState::WaitingForFrame {
                            expected_length: frame_length,
                        };
                    }
                    ReadState::WaitingForFrame { expected_length } => {
                        while self.buffer.len() < *expected_length as usize {
                            let mut temp_buffer = [0u8; 1024];
                            let bytes_read = self
                                .reader
                                .read(&mut temp_buffer)
                                .await
                                .map_err(FrameError::Read)?;

                            if bytes_read == 0 {
                                debug!("Unable to read, is the connection closed?");
                                return Err(FrameError::InvalidFrameLength(0)); // Connection is closed
                            }

                            self.buffer.extend_from_slice(&temp_buffer[..bytes_read]);
                        }

                        let frame_data = self.buffer[..*expected_length as usize].to_vec();

                        self.buffer.drain(..*expected_length as usize);

                        self.state = ReadState::WaitingForLength;

                        // Successfully read frame data
                        // Frame read successfully (verbose logging removed)

                        // Parse frame first
                        debug!("Frame data size: {}", frame_data.len());
                        let frame_result = Frame::try_from(frame_data.as_slice());

                        return frame_result;
                    }
                }
            }
        }
    }

    fn is_connected(&self) -> impl std::future::Future<Output = bool> + Send {
        async move {
            self.reader
                .get_ref()
                .ready(Interest::READABLE)
                .await
                .is_ok()
        }
    }
}
