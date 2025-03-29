pub mod utils {
    use std::io;
    use serde_derive::{Deserialize, Serialize};
    use std::net::TcpStream;
    use std::error::Error;
    use std::io::{Read, Write};
    
    
    /// This type is used to wrap data sent between clients and server.
    #[derive(Serialize, Deserialize)]
    pub enum MessageType {
        Text(String),
        Image(Vec<u8>),
        File(String, Vec<u8>),
    }


    /// This function uses stream to receive data and save them in a vector of bytes.
    pub fn receive_bytes(mut stream: &TcpStream) -> Result<Vec<u8>, io::Error> {
        let mut bytes_len_buf = [0u8; 4];
        stream.read_exact(&mut bytes_len_buf)?;
        let bytes_len = u32::from_be_bytes(bytes_len_buf) as usize;
        let mut buffer = vec![0u8; bytes_len];
        stream.read_exact(&mut buffer)?;
        Ok(buffer)
    }


    /// This function receives an array of bytes and sends them using stream.
    pub fn send_bytes(mut stream: &TcpStream, bytes: &[u8]) -> Result<(), Box<dyn Error>> {
        let len = bytes.len() as u32;
        stream.write(&len.to_be_bytes())?;
        stream.write_all(bytes)?;
        Ok(())
    }
}


pub use utils::{MessageType, receive_bytes, send_bytes};
