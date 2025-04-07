pub mod utils {
    use std::io;
    use serde_derive::{Deserialize, Serialize};
    use std::net::TcpStream;
    use std::io::{Read, Write};
    use thiserror::Error;
    
    
    /// This type is used to wrap data sent between clients and server.
    #[derive(Serialize, Deserialize)]
    pub enum MessageType {
        Text(String),
        Image(Vec<u8>),
        File(String, Vec<u8>),
    }


    ///
    #[derive(Error, Debug)]
    pub enum BytesSendReceiveError {
        #[error("Sending bytes failed.")]
        SendFailed(#[source] io::Error),
        #[error("Receiving bytes failed.")]
        ReceiveFailed(#[source] io::Error),
        #[error("Receiving timed out.")]
        ReceiveTimeout(#[source] io::Error)
    }


    /// This function uses stream to receive data and save them in a vector of bytes.
    pub fn receive_bytes(mut stream: &TcpStream) -> Result<Vec<u8>, BytesSendReceiveError> {
        let mut bytes_len_buf = [0u8; 4];
        stream.read_exact(&mut bytes_len_buf).map_err(|e| {
            if e.kind() == io::ErrorKind::TimedOut {
                return BytesSendReceiveError::ReceiveTimeout(e);
            } else {
                return BytesSendReceiveError::ReceiveFailed(e);
            }
        })?;
        let bytes_len = u32::from_be_bytes(bytes_len_buf) as usize;
        let mut buffer = vec![0u8; bytes_len];
        stream.read_exact(&mut buffer).map_err(BytesSendReceiveError::ReceiveFailed)?;
        Ok(buffer)
    }


    /// This function receives an array of bytes and sends them using stream.
    pub fn send_bytes(mut stream: &TcpStream, bytes: &[u8]) -> Result<(), BytesSendReceiveError> {
        let len = bytes.len() as u32;
        stream.write(&len.to_be_bytes()).map_err(BytesSendReceiveError::SendFailed)?;
        stream.write_all(bytes).map_err(BytesSendReceiveError::SendFailed)?;
        Ok(())
    }
}


pub use utils::{MessageType, receive_bytes, send_bytes};
