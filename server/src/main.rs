use std::error::Error;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::thread;
use clap::{arg, Command};
use log::{info, error};
use anyhow::{Context, Result, anyhow};

use shared::{receive_bytes, send_bytes};


/// This function runs server.
fn run_server(socket_address: &str) -> Result<()> {
    let listener = TcpListener::bind(socket_address).context("TcpListener failed to bind to a socket address.")?;
    let clients: Arc<Mutex<HashMap<SocketAddr, TcpStream>>> = Arc::new(Mutex::new(HashMap::new()));
    
    // Create a new stream for each incomming connection.
    for client_stream in listener.incoming() {
        let client_stream = client_stream.context("Looping over incomming connections gave an Error.")?;
        let client_stream_cloned = client_stream.try_clone().context("Cloning TcpStream failed.")?;
        let client_address = client_stream.peer_addr().context("Failed to obtain the address of an incomming connection.")?;
        let clients_cloned = Arc::clone(&clients);
        {
            // Add a new stream to a hash map. The socket address is key.
            let mut lock = clients_cloned.lock().map_err(|e| anyhow!("Failed to lock the HashMap holding sockets: {}", e))?;
            lock.insert(client_address.clone(), client_stream);
        }
        // For each incomming connection, there is a separate thread.
        thread::spawn(move || {
            if let Err(e) = handle_client(client_address, client_stream_cloned, clients_cloned) {
                error!("Thread stopped executing due to an error: {}", e);
            };
        });
    }
    Ok(())
}


/// This function is executed in a separate thread for each incomming connection.
fn handle_client(client_address: SocketAddr, client_stream: TcpStream, clients: Arc<Mutex<HashMap<SocketAddr, TcpStream>>>) -> Result<()> {
    loop {
        let received_bytes = receive_bytes(&client_stream).context("Failed when receiving bytes.")?;
        let lock = clients.lock().map_err(|e| anyhow!("Failed to lock the HashMap holding sockets: {}", e))?;
        
        // Send received data to all clients except the one from which the data were received.
        for address in lock.keys() {
            if *address != client_address {
                let destination_stream = lock.get(address).ok_or_else(|| anyhow!("Address not found in HashMap."))?;
                send_bytes(destination_stream, &received_bytes).context("Failed when sending bytes.")?;
            }
        }
    }
}


fn main() -> Result<()>  {
    env_logger::init();

    let matches = Command::new("Server")
        .about("Runs server")
        .arg(arg!(--address <SOCKET>).default_value("127.0.0.1:11111"))
        .get_matches();

    let socket_address = matches.get_one::<String>("address").ok_or_else(|| anyhow!("There is always a value."))?;

    info!("Starting server...");
    run_server(socket_address).context("Server stopped running because of an error.")?;
    info!("Exiting server...");

    Ok(())
}
