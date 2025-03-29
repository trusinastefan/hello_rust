use std::error::Error;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::thread;
use clap::{arg, Command};
use log::{info, error};

use shared::{receive_bytes, send_bytes};


/// This function runs server.
fn run_server(socket_address: &str) -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind(socket_address)?;
    let clients: Arc<Mutex<HashMap<SocketAddr, TcpStream>>> = Arc::new(Mutex::new(HashMap::new()));
    
    // Create a new stream for each incomming connection.
    for client_stream in listener.incoming() {
        let client_stream = client_stream?;
        let client_stream_cloned = client_stream.try_clone()?;
        let client_address = client_stream.peer_addr()?;
        let clients_cloned = Arc::clone(&clients);
        {
            // Add the new stream to a hash map. The socket address is key.
            match clients_cloned.lock() {
                Ok(mut lock) => {
                    lock.insert(client_address.clone(), client_stream);
                },
                Err(e) => {
                    return Err(format!("{}", e).into());
                }
            };
        }
        // For each incomming connection, there is a separate thread.
        thread::spawn(move || {
            if let Err(e) = handle_client(client_address, client_stream_cloned, clients_cloned) {
                error!("{}", e);
            };
        });
    }
    Ok(())
}


/// This function is executed in a separate thread for each incomming connection.
fn handle_client(client_address: SocketAddr, client_stream: TcpStream, clients: Arc<Mutex<HashMap<SocketAddr, TcpStream>>>) -> Result<(), Box<dyn Error>> {
    loop {
        let received_bytes = receive_bytes(&client_stream)?;
        match clients.lock() {
            Ok(lock) => {
                // Send received data to all clients except the one from which the data were received.
                for address in lock.keys() {
                    if *address != client_address {
                        match lock.get(address) {
                            Some(destination_stream) => {
                                send_bytes(destination_stream, &received_bytes)?;
                            },
                            None => {
                                return Err(format!("Address not found in HashMap!").into());
                            }
                        };
                    }
                }
            },
            Err(e) => {
                return Err(format!("{}", e).into());
            }
        };
    }
}


fn main() {
    env_logger::init();

    let matches = Command::new("Server")
        .about("Runs server")
        .arg(arg!(--address <SOCKET>).default_value("127.0.0.1:11111"))
        .get_matches();

    let socket_address = matches.get_one::<String>("address").expect("There is always a value.");

    info!("Starting server!");
    match run_server(socket_address) {
        Ok(()) => {
            info!("Exiting server!");
        },
        Err(e) => {
            error!("{}", e);
        }
    };
}
