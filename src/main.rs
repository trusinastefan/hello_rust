use std::{env, io, str};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::thread;
use chrono::Local;
use serde_cbor::{to_vec, from_slice};
use serde_derive::{Deserialize, Serialize};
use std::fs::{self, File};
use std::path::Path;
use std::time::Duration;


/********************************
 * Utility functions and types. *
 ********************************/

 /// This type is used to wrap data sent between clients and server.
#[derive(Serialize, Deserialize)]
enum MessageType {
    Text(String),
    Image(Vec<u8>),
    File(String, Vec<u8>),
}


/// This function uses stream to receive data and save them in a vector of bytes.
fn receive_bytes(mut stream: &TcpStream) -> Result<Vec<u8>, io::Error> {
    let mut bytes_len_buf = [0u8; 4];
    stream.read_exact(&mut bytes_len_buf)?;
    let bytes_len = u32::from_be_bytes(bytes_len_buf) as usize;
    let mut buffer = vec![0u8; bytes_len];
    stream.read_exact(&mut buffer)?;
    Ok(buffer)
}


/// This function receives an array of bytes and sends them using stream.
fn send_bytes(mut stream: &TcpStream, bytes: &[u8]) -> Result<(), Box<dyn Error>> {
    let len = bytes.len() as u32;
    stream.write(&len.to_be_bytes())?;
    stream.write_all(bytes)?;
    Ok(())
}


/****************
 * Server code. *
 ****************/

/// Run server.
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
                eprintln!("{}", e);
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


/****************
 * Client code. *
 ****************/

fn run_client(socket_address: &str) -> Result<(), Box<dyn Error>> {
    // A shared variable. If user types .quit, this variable is set to false.
    let continue_running: Arc<Mutex<bool>> = Arc::new(Mutex::new(true));
    let continue_running_cloned = Arc::clone(&continue_running);
    let stream = TcpStream::connect(socket_address)?;
    // Reading will timeout regularly so that the "receiver" thread can check regularly the value of continue_running.
    stream.set_read_timeout(Some(Duration::from_secs(1)))?;
    let stream_cloned = stream.try_clone()?;
    // This thread will handle data received through stream.
    let handle = thread::spawn(move || {
        // In the loop, it regularly tries to read from stream.
        loop {
            match receive_bytes(&stream) {
                Ok(received_bytes) => {
                    if let Err(e) = handle_received_data_in_client(&received_bytes) {
                        println!("Cannot handle received data: {}", e);
                        continue;
                    };
                },
                // Check continue_running.
                Err(ref e) if e.kind() == io::ErrorKind::TimedOut => {
                    match continue_running_cloned.lock() {
                        Ok(lock) => {
                            if *lock == false {
                                break;
                            }
                        },
                        Err(e) => {
                            eprintln!("{}", e);
                            break;
                        }
                    }
                },
                Err(e) => {
                    eprintln!("{}", e);
                    break;
                }
            };
        }
    });
    // Loop for getting user input and sending data according to this input.
    loop {
        let user_input = get_line_from_user()?;
        if user_input.trim() == ".quit" {
            match continue_running.lock() {
                Ok(mut lock) => {
                    *lock = false;
                },
                Err(e) => {
                    eprintln!("{}", e);
                }
            }
            break;
        }
        // Based on user input, prepare a vector of bytes that should be sent.
        let bytes = match prepare_data_based_on_user_input(user_input) {
            Ok(b) => b,
            Err(e) => {
                println!("{}", e);
                continue;
            }
        };
        // Send bytes - direction server.
        if let Err(e) = send_bytes(&stream_cloned, &bytes) {
            eprintln!("{}", e);
            break;
        }
    }
    if let Err(_e) = handle.join() {
        eprintln!("Error encountered when joining threads in client!");
    }
    Ok(())
}


// Get user input from stdin.
fn get_line_from_user() -> Result<String, Box<dyn Error>> {
    let mut input_str = String::new();
    io::stdin().read_line(&mut input_str)?;
    Ok(input_str.trim().to_string())
}

// When data are received, run this function.
fn handle_received_data_in_client(received_bytes: &Vec<u8>) -> Result<(), Box<dyn Error>> {
    let message: MessageType = from_slice(received_bytes)?;
    // The incomming message can have three types.
    match message {
        MessageType::File(name, bytes) => {
            println!("Receiving {}", &name);
            save_file("files".to_string(), name, bytes)?;
        },
        MessageType::Image(bytes) => {
            println!("Receiving image ...");
            let now = Local::now().format("%Y_%m_%d_%H_%M_%S").to_string();
            let name = format!("{}.png", now);
            save_file("images".to_string(), name, bytes)?;
        },
        MessageType::Text(text) => {
            println!("{}", text);
        }
    }
    Ok(())
}


// Create a file and write bytes to it.
fn save_file(dir: String, name: String, bytes: Vec<u8>) -> Result<(), Box<dyn Error>> {
    let mut file = File::create(format!("{}\\{}", dir, name))?;
    file.write(&bytes)?;
    Ok(())
}


// Based on what user typed into stdin, create a MessageType object and serialize it.
fn prepare_data_based_on_user_input(user_input: String) -> Result<Vec<u8>, Box<dyn Error>> {
    let message: MessageType;
    if user_input.starts_with(".file ") {
        message = get_file_message(user_input)?;
    } else if user_input.starts_with(".image ") {
        message = get_image_message(user_input)?;

    } else {
        message = MessageType::Text(user_input);
    }

    let bytes = to_vec(&message)?;
    Ok(bytes)
}


// If the user's command is of type ".file", create a MessageType object of type File.
fn get_file_message(user_input: String) -> Result<MessageType, Box<dyn Error>> {
    let path_str = match user_input.strip_prefix(".file ") {
        Some(p) => p,
        None => {
            return Err("Could not strip prefix!".into());
        }
    };
    let bytes = match fs::read(path_str) {
        Ok(b) => b,
        Err(_e) => {
            return Err("Cannot read the file!".into());
        }
    };
    let file_name = match Path::new(path_str).file_name() {
        Some(n) => n.to_string_lossy().into_owned(),
        None => {
            return Err("Cannot parse file name!".into());
        }
    };
    Ok(MessageType::File(file_name, bytes))
}


// If the user's command is of type ".file", create a MessageType object of type Image.
fn get_image_message(user_input: String) -> Result<MessageType, Box<dyn Error>> {
    let path_str = match user_input.strip_prefix(".image ") {
        Some(p) => p,
        None => {
            return Err("Could not strip prefix!".into());
        }
    };
    match Path::new(path_str).extension() {
        Some(e) => {
            if e != "png" {
                return Err("The file's extention is not png'!".into());
            }
        },
        None => {
            return Err("Cannot parse file name!".into());
        }
    };
    let bytes = match fs::read(path_str) {
        Ok(b) => b,
        Err(_e) => {
            return Err("Cannot read the file!".into());
        }
    };
    Ok(MessageType::Image(bytes))
}


/******************
 * Main function. *
 ******************/

fn main() {
    let args: Vec<String> = env::args().collect();
    // If no address specified by user, use the default address.
    let socket_address: &str;
    if args.len() == 4 && args[2] == "--address" {
        socket_address = &args[3];
    } else {
        socket_address = "127.0.0.1:11111";
    }
    if args.len() == 1 {
        println!("Command not specified. Exiting program...");
    } else if args.len() >= 2 && args[1] == "run-server" {
        println!("Starting server!");
        match run_server(socket_address) {
            Ok(()) => {
                println!("Exiting server!");
            },
            Err(e) => {
                eprintln!("{}", e);
            }
        };
    } else if args.len() >= 2 && args[1] == "run-client" {
        println!("Starting client!");
        match run_client(socket_address) {
            Ok(()) => {
                println!("Exiting client!");
            },
            Err(e) => {
                eprintln!("{}", e);
            }
        }
    } else {
        println!("Cannot help you, sorry...");
    }
    
}
