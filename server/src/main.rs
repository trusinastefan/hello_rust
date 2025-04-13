use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use std::net::SocketAddr;
use std::sync::Arc;
use std::collections::HashMap;
use clap::{arg, Command};
use log::{info, error};
use anyhow::{Context, Result, anyhow};

use shared::{MessageType,  receive_bytes, send_bytes, receive_message, send_message};


type SharedWriteHalf = Arc<Mutex<OwnedWriteHalf>>;


/// This function runs server.
async fn run_server(socket_address: &str) -> Result<()> {
    let listener = TcpListener::bind(socket_address).await.context("TcpListener failed to bind to a socket address.")?;
    let client_writers: Arc<Mutex<HashMap<SocketAddr, SharedWriteHalf>>> = Arc::new(Mutex::new(HashMap::new()));
    
    loop {
        // Create a new stream for each incomming connection.
        let (client_stream, client_address) = listener.accept().await.context("Failed to accept a new connection from a client.")?;
        // Split each stream into a reader and a writer.
        let (client_reader, client_writer) = client_stream.into_split();
        // Clone reader hash map.
        let client_writers_cloned = Arc::clone(&client_writers);
        // Add writer to respective hash maps. The socket address is key.
        {
            let mut lock = client_writers.lock().await;
            lock.insert(client_address.clone(), Arc::new(Mutex::new(client_writer)));
        }
        // For each incomming connection, there is a separate async task.
        tokio::spawn(async move {
            if let Err(e) = handle_client(client_address, client_reader, client_writers_cloned).await {
                error!("Client handler on server stopped executing due to an error: {}", e);
            };
        });
    }
    Ok(())
}


/// This function is executed as a separate async task for each incomming connection.
async fn handle_client(client_address: SocketAddr, mut client_reader: OwnedReadHalf, client_writers: Arc<Mutex<HashMap<SocketAddr, SharedWriteHalf>>>) -> Result<()> {
    // Try to authenticate user. If not successful, the connection will be dropped.
    let username = match authenticate_user(&mut client_reader, &client_address, &client_writers).await {
        Some(name) => name,
        None => {
            return Ok(());
        }
    };
    loop {
        // Wait for data from a client.
        let received_bytes = receive_bytes(&mut client_reader).await.context("Failed when receiving bytes.")?;

        // Send received data to all clients except the one from which the data were received.
        let lock = client_writers.lock().await;
        for address in lock.keys() {
            if *address != client_address {
                let shared_writer = lock.get(address).ok_or_else(|| anyhow!("Address not found in HashMap."))?;
                let mut lock_writer = shared_writer.lock().await;
                send_bytes(&mut *lock_writer, &received_bytes).await.context("Failed when sending bytes.")?;
            }
        }
    }
}


///
async fn authenticate_user(reader: &mut OwnedReadHalf, client_address: &SocketAddr, client_writers: &Arc<Mutex<HashMap<SocketAddr, SharedWriteHalf>>>) -> Option<String> {
    // Wait for authentication request message.
    let (action, username, password) = match receive_message(reader).await {
                
        // Data received and passed to the handler.
        Ok(MessageType::AuthRequest(action, username, password)) => {
            info!("Received authentication request from {}.", &username);
            (action, username, password)
        },

        // Incorrect MessageType. This should never happen.
        Ok(_) => {
            error!("Incorrect message type received from server.");
            return None;
        }
        
        // Error while reading.
        Err(e) => {
            error!("Error while waiting for an authentication request: {}", e);
            return None;
        }
    };

    // Authenticate and return success status.
    let (auth_successful, message_from_server) = match handle_auth_request(&action, &username, &password).await {
        Ok(message) => message,
        Err(e) => {
            error!("Error while handling authentication request: {}", e);
            return None;
        }
    };

    // Send authentication response message back to the user.
    let auth_response_message = MessageType::AuthResponse(auth_successful, message_from_server);
    let lock = client_writers.lock().await;
    let shared_writer = match lock.get(client_address) {
        Some(w) => w,
        None => {
            error!("Address not found in HashMap.");
            return None;
        }
    };
    let mut lock_writer = shared_writer.lock().await;
    match send_message(&mut *lock_writer, &auth_response_message).await {
        Ok(_) => {
            // Only if we return 'true' here, will the authentication process be successful.
            info!("Authentication status: {}. Sending request back to user.", &auth_successful);
            return Some(username);
        },
        Err(e) => {
            error!("Error while sending authentication response: {}", e);
            return None;
        }
    }
}


/// Based on parameters, try to either register or authenticate user. Produce a response message for client.
async fn handle_auth_request(action: &String, username: &String, password: &String) -> Result<(bool, String)> {
    Ok((true, "Authentication succeeded.".to_string()))
}


#[tokio::main]
async fn main() -> Result<()>  {
    env_logger::init();

    let matches = Command::new("Server")
        .about("Runs server")
        .arg(arg!(--address <SOCKET>).default_value("127.0.0.1:11111"))
        .get_matches();

    let socket_address = matches.get_one::<String>("address").ok_or_else(|| anyhow!("There is always a value."))?;

    info!("Starting server...");
    run_server(socket_address).await.context("Server stopped running because of an error.")?;
    info!("Exiting server...");

    Ok(())
}
