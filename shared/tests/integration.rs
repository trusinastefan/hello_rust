use shared::*;
use tokio::net::{tcp::{OwnedReadHalf, OwnedWriteHalf}, TcpListener, TcpStream};
use anyhow::Result;


/// Prepare one reader and one writer. A connection should exist between them.
async fn prepare_reader_and_writer(socket_address_of_server: &str) -> Result<(OwnedReadHalf, OwnedWriteHalf)> {
    let listener_on_server = TcpListener::bind(socket_address_of_server).await.unwrap();
    let stream_on_client = TcpStream::connect(socket_address_of_server).await.unwrap();
    let (_, writer_on_client) = stream_on_client.into_split();
    let (stream_on_server, _) = listener_on_server.accept().await.unwrap();
    let (reader_on_server, _) = stream_on_server.into_split();
    Ok((reader_on_server, writer_on_client))
}

#[tokio::test]
async fn test_sending_and_receiving_bytes() {

    // Prepare reader and writer.
    let socket_address_of_server = "127.0.0.1:11111";
    let (mut reader_on_server, mut writer_on_client) = prepare_reader_and_writer(socket_address_of_server).await.unwrap();    

    // Prepare a payload from bytes that will be sent and again received.
    let test_string = "This is a test string.";
    let test_payload = test_string.as_bytes();

    //Send and receive payload.
    send_bytes(&mut writer_on_client, test_payload).await.unwrap();
    let received_payload = receive_bytes(&mut reader_on_server).await.unwrap();
    
    // Check if received payload matches the sent payload.
    assert_eq!(received_payload, test_payload.to_vec());
}

#[tokio::test]
async fn test_sending_and_receiving_messages() {

    // Prepare reader and writer.
    let socket_address_of_server = "127.0.0.1:22222";
    let (mut reader_on_server, mut writer_on_client) = prepare_reader_and_writer(socket_address_of_server).await.unwrap();    

    // Prepare a test payload message that will be sent and again received.
    let test_message = MessageType::Text("This is a test string.".to_string());

    //Send and receive payload.
    send_message(&mut writer_on_client, &test_message).await.unwrap();
    let received_message = receive_message(&mut reader_on_server).await.unwrap();
    
    // Check if received payload matches the sent payload.
    assert_eq!(test_message, received_message);
}
