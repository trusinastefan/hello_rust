pub mod utils {
    use std::io;
    use serde_derive::{Deserialize, Serialize};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
    use thiserror::Error;
    use anyhow::{Context, Result};
    use serde_cbor::{to_vec, from_slice};
    
    
    /// This type is used to wrap data sent to server and other clients.
    #[derive(Serialize, Deserialize)]
    pub enum MessageType {
        Text(String),
        Image(Vec<u8>),
        File(String, Vec<u8>),
        AuthRequest(String, String, String),
        AuthResponse(bool, String)
    }


    ///
    #[derive(Error, Debug)]
    pub enum BytesSendReceiveError {
        #[error("Sending bytes failed.")]
        SendFailed(#[source] io::Error),
        #[error("Receiving bytes failed.")]
        ReceiveFailed(#[source] io::Error)
    }


    /// This function uses stream to receive data and save them in a vector of bytes.
    pub async fn receive_bytes(stream_reader: &mut OwnedReadHalf) -> Result<Vec<u8>, BytesSendReceiveError> {
        let mut bytes_len_buf = [0u8; 4];
        stream_reader.read_exact(&mut bytes_len_buf).await.map_err(BytesSendReceiveError::ReceiveFailed)?;
        let bytes_len = u32::from_be_bytes(bytes_len_buf) as usize;
        let mut buffer = vec![0u8; bytes_len];
        stream_reader.read_exact(&mut buffer).await.map_err(BytesSendReceiveError::ReceiveFailed)?;
        Ok(buffer)
    }


    /// This function receives an array of bytes and sends them using stream.
    pub async fn send_bytes(stream_writer: &mut OwnedWriteHalf, bytes: &[u8]) -> Result<(), BytesSendReceiveError> {
        let len = bytes.len() as u32;
        stream_writer.write(&len.to_be_bytes()).await.map_err(BytesSendReceiveError::SendFailed)?;
        stream_writer.write_all(bytes).await.map_err(BytesSendReceiveError::SendFailed)?;
        Ok(())
    }


    /// This function uses stream to receive data and turn them into a message.
    pub async fn receive_message(mut stream_reader: &mut OwnedReadHalf) -> Result<MessageType> {
        let bytes = receive_bytes(&mut stream_reader).await.context("Failed when receiving bytes.")?;
        let message: MessageType = from_slice(&bytes).context("Failed to turn bytes into MessageType.")?;
        Ok(message)
    }
    

    /// This function receives a message, turns it into bytes and sends them using stream.
    pub async fn send_message(stream_writer: &mut OwnedWriteHalf, message: &MessageType) -> Result<()> {
        let bytes = to_vec(&message).context("Failed to turn message into a vector of bytes.")?;
        send_bytes(stream_writer, &bytes).await.context("Failed when sending bytes.")?;
        Ok(())
    }
}


pub use utils::{MessageType, BytesSendReceiveError, receive_bytes, send_bytes, receive_message, send_message};
