use std::fs::{self, File};
use std::net::TcpStream;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::error::Error;
use std::{io, thread};
use std::io::Write;
use clap::{arg, Command};
use chrono::Local;
use serde_cbor::{to_vec, from_slice};
use std::time::Duration;
use log::{info, error};
use anyhow::{Context, Result, anyhow};

use shared::{receive_bytes, send_bytes, MessageType};

/// This is the main client function.
/// Its main thread waits for a user input and sends it to server.
/// Another spawned thread listens on a socket for incoming messages and prints them in console.
fn run_client(socket_address: &str) -> Result<()> {
    // A shared variable. If user types .quit, this variable is set to false.
    let continue_running: Arc<Mutex<bool>> = Arc::new(Mutex::new(true));
    let continue_running_cloned = Arc::clone(&continue_running);
    let stream = TcpStream::connect(socket_address).context("Failed to connect to a server.")?;
    // Reading will timeout regularly so that the "receiver" thread can check regularly the value of continue_running.
    stream.set_read_timeout(Some(Duration::from_secs(1))).context("Failed to set read timeout.")?;
    let stream_cloned = stream.try_clone().context("Failed to clone TcpStream.")?;
    
    // This thread will handle data received through stream.
    let handle = thread::spawn(move || {
        
        // In the loop, it regularly tries to read from stream.
        loop {
            match receive_bytes(&stream) {
                Ok(received_bytes) => {
                    if let Err(e) = handle_received_data_in_client(&received_bytes) {
                        error!("Cannot handle received data: {}", e);
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
                            error!("{}", e);
                            break;
                        }
                    }
                },

                Err(e) => {
                    error!("{}", e);
                    break;
                }
            };
        }
    });

    // Loop for getting user input and sending data according to this input.
    loop {
        // Get input.
        let user_input = get_line_from_user()?;

        // The .quit commands causes the client program to quit.
        if user_input.trim() == ".quit" {
            match continue_running.lock() {
                Ok(mut lock) => {
                    *lock = false;
                },
                Err(e) => {
                    error!("{}", e);
                }
            }
            break;
        }

        // Based on user input, prepare a vector of bytes that should be sent.
        let bytes = match prepare_data_based_on_user_input(user_input) {
            Ok(b) => b,
            Err(e) => {
                error!("{}", e);
                continue;
            }
        };

        // Send bytes - direction server.
        if let Err(e) = send_bytes(&stream_cloned, &bytes) {
            error!("{}", e);
            break;
        }
    }

    if let Err(_e) = handle.join() {
        error!("Error encountered when joining threads in client!");
    }
    Ok(())
}


/// Get user input from stdin.
fn get_line_from_user() -> Result<String, Box<dyn Error>> {
    let mut input_str = String::new();
    io::stdin().read_line(&mut input_str)?;
    Ok(input_str.trim().to_string())
}


/// Function for handling received data.
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


/// Create a file and write bytes to it.
fn save_file(dir: String, name: String, bytes: Vec<u8>) -> Result<(), Box<dyn Error>> {
    let mut file = File::create(format!("{}\\{}", dir, name))?;
    file.write(&bytes)?;
    Ok(())
}


/// Based on what user typed into stdin, create a MessageType object and serialize it.
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


/// If the user's command is of type ".file", create a MessageType object of type File.
fn get_file_message(user_input: String) -> Result<MessageType> {
    let path_str = user_input.strip_prefix(".file ").ok_or_else(|| anyhow!("Failed to strip the '.file' prefix."))?;
    
    let bytes = fs::read(path_str).context("Failed to read file.")?;

    let file_name = Path::new(path_str).file_name().context("Failed to parse filename.")?;
    let file_name = file_name.to_string_lossy().into_owned();
    
    Ok(MessageType::File(file_name, bytes))
}


/// If a user's command is of type ".image", create a MessageType object of type Image.
fn get_image_message(user_input: String) -> Result<MessageType> {
    let path_str = user_input.strip_prefix(".image ").ok_or_else(|| anyhow!("Failed to strip the '.image' prefix."))?;

    if "png" != Path::new(path_str).extension().ok_or_else(|| anyhow!("Cannot parse extention from filename."))? {
        return Err(anyhow!("The file's extention is not '.png'."));
    }

    let bytes = fs::read(path_str).context("Failed to read file.")?;

    Ok(MessageType::Image(bytes))
}


fn main() -> Result<()> {
    env_logger::init();

    let matches = Command::new("Client")
        .about("Runs client")
        .arg(arg!(--address <SOCKET>).default_value("127.0.0.1:11111"))
        .get_matches();

    let socket_address = matches.get_one::<String>("address").ok_or_else(|| anyhow!("There is always a value."))?;

    info!("Starting client...");
    run_client(socket_address).context("Client stopped running because of an error.")?;
    info!("Exiting client!...");

    Ok(())
}
