use tokio::fs::{self, File};
use tokio::net::TcpStream;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::io::AsyncWriteExt;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;
use clap::{arg, Command};
use chrono::Local;
use tokio::time::{Duration, timeout};
use log::{info, error};
use anyhow::{Context, Result, anyhow};

use shared::{MessageType, receive_message, send_message};


/// This is the main client function.
/// Its main thread waits for a user input and sends it to server.
/// Another spawned thread listens on a socket for incoming messages and prints them in console.
async fn run_client(socket_address: &str) -> Result<()> {
    
    // Try to connect to server and get a stream object.
    let stream = TcpStream::connect(socket_address).await.context("Failed to connect to a server.")?;
    // Split stream into reader and writer.
    let (mut reader, mut writer) = stream.into_split();
    
    // Try to authenticate user. If not successful, exit.
    let auth_successful = authenticate_user(&mut reader, &mut writer).await.context("Authentification failed.")?;
    if !auth_successful {
        return Ok(());
    }
    
    // A shared variable. If user types .quit, this variable is set to false.
    let continue_running: Arc<Mutex<bool>> = Arc::new(Mutex::new(true));
    let continue_running_cloned = Arc::clone(&continue_running);
    
    // This thread will handle data received through stream.
    let handle = tokio::spawn(async move {
        
        // In the loop, it regularly tries to read from stream.
        loop {
            match timeout(Duration::from_secs(3), receive_message(&mut reader)).await {
                
                // Data received and passed to the handler.
                Ok(Ok(received_message)) => {
                    if let Err(e) = handle_received_data_in_client(received_message).await {
                        error!("Cannot handle received data: {}", e);
                        continue;
                    };
                },
                
                // Error while reading.
                Ok(Err(e)) => {
                    return Err(anyhow!("Error while reading: {}", e));
                }
                
                // Reading will timeout regularly so that the "receiver" async task can check regularly the value of continue_running.
                Err(_) => {
                    let lock_continue_running = continue_running_cloned.lock().await;
                    // Check continue_running.
                    if !(*lock_continue_running) {
                        break;
                    }
                },
            };
        };
        Ok(())
    });

    // Loop for getting user input and sending data according to this input.
    loop {
        // Get input.
        let user_input = get_line_from_user().await.context("Failed to get user input.")?;

        // The .quit commands causes the client program to quit.
        if user_input.trim() == ".quit" {
            let mut lock_continue_running = continue_running.lock().await;
            *lock_continue_running = false;
            break;
        }

        // Based on user input, prepare a vector of bytes that should be sent.
        let message = match prepare_message_based_on_user_input(user_input).await {
            Ok(m) => m,
            Err(e) => {
                error!("There was a problem processing user input: {}", e);
                continue;
            }
        };

        // Send bytes - direction server.
        send_message(&mut writer, &message).await.context("Failed to send message.")?;
    };
    let _ = handle.await.map_err(|e| anyhow!("Error occured in spawned thread: {:?}", e))?;
    Ok(())
}


/// Register or login user. In both cases, a name and a password are required.
async fn authenticate_user(reader: &mut OwnedReadHalf, writer: &mut OwnedWriteHalf) -> Result<bool> {
    // Find out if user wants to register or login.
    println!("Do you want to register or login? (R/L)");
    let action = get_line_from_user().await.context("Failed to get user action.")?;
    if action != "R" && action != "L" {
        println!("Invalid input! You must type either 'R' or 'L'!");
        return Ok(false)
    }
    // Get username and password.
    println!("Username:");
    let username = get_line_from_user().await.context("Failed to get username.")?;
    println!("Password:");
    let password = get_line_from_user().await.context("Failed to get password.")?;

    // Create and send authentication request message.
    let request_message = MessageType::AuthRequest(action, username, password);
    send_message(writer, &request_message).await.context("Failed to send auth request.")?;

    // Wait for authentication response message.
    match timeout(Duration::from_secs(5), receive_message(reader)).await {
                
        // Data received and passed to the handler.
        Ok(Ok(MessageType::AuthResponse(auth_successful, message_from_server))) => {
            if auth_successful {
                println!("Authentication succesfull: {}", message_from_server);
                return Ok(true)
            } else {
                println!("Authentication not succesfull: {}", message_from_server);
                return Ok(false)
            }
        },

        // Incorrect MessageType. This should never happen.
        Ok(Ok(_)) => {
            return Err(anyhow!("Incorrect message type received from server."));
        }
        
        // Error while reading.
        Ok(Err(e)) => {
            return Err(anyhow!("Error while waiting for an authentication response: {}", e));
        }
        
        // Waiting for authentication response timeout.
        Err(_) => {
            println!("Authentication timeout. The server took too long to respond.");
            return Ok(false);
        },
    };
}


/// Get user input from stdin.
async fn get_line_from_user() -> Result<String> {
    let mut input_str = String::new();
    std::io::stdin().read_line(&mut input_str).context("Failed to read from standard input.")?;
    Ok(input_str.trim().to_string())
}


/// Function for handling received data.
async fn handle_received_data_in_client(message: MessageType) -> Result<()> {
    
    // The behaviour will be based on the message type.
    match message {
        MessageType::File(name, bytes) => {
            println!("Receiving {}...", &name);
            save_file("files".to_string(), name, bytes).await.context("Failed to save file to directory 'files'.")?;
        },
        MessageType::Image(bytes) => {
            println!("Receiving image ...");
            let now = Local::now().format("%Y_%m_%d_%H_%M_%S").to_string();
            let name = format!("{}.png", now);
            save_file("images".to_string(), name, bytes).await.context("Failed to save '.png' image to directory 'images'.")?;
        },
        MessageType::Text(text) => {
            println!("{}", text);
        },
        // To all other message types, react will we not.
        _ => {}
    }

    Ok(())
}


/// Create a file and write bytes to it.
async fn save_file(dir: String, name: String, bytes: Vec<u8>) -> Result<()> {
    let mut file = File::create(format!("{}\\{}", dir, name)).await.context("Failed to create file.")?;
    file.write(&bytes).await.context("Failed to write bytes into file.")?;
    Ok(())
}


/// Based on what user typed into stdin, create a MessageType object and serialize it.
async fn prepare_message_based_on_user_input(user_input: String) -> Result<MessageType> {
    let message: MessageType;
    if user_input.starts_with(".file ") {
        message = get_file_message(user_input).await.context("The '.file' command seems to be invalid.")?;
    } else if user_input.starts_with(".image ") {
        message = get_image_message(user_input).await.context("The '.image' command seems to be invalid.")?;
    } else {
        message = MessageType::Text(user_input);
    }

    Ok(message)
}


/// If the user's command is of type ".file", create a MessageType object of type File.
async fn get_file_message(user_input: String) -> Result<MessageType> {
    let path_str = user_input.strip_prefix(".file ").ok_or_else(|| anyhow!("Failed to strip the '.file' prefix."))?;
    let bytes = fs::read(path_str).await.context("Failed to read file.")?;
    let file_name = Path::new(path_str).file_name().context("Failed to parse filename.")?;
    let file_name = file_name.to_string_lossy().into_owned();
    
    Ok(MessageType::File(file_name, bytes))
}


/// If a user's command is of type ".image", create a MessageType object of type Image.
async fn get_image_message(user_input: String) -> Result<MessageType> {
    let path_str = user_input.strip_prefix(".image ").ok_or_else(|| anyhow!("Failed to strip the '.image' prefix."))?;

    if "png" != Path::new(path_str).extension().ok_or_else(|| anyhow!("Cannot parse extention from filename."))? {
        return Err(anyhow!("The file's extention is not '.png'."));
    }

    let bytes = fs::read(path_str).await.context("Failed to read file.")?;

    Ok(MessageType::Image(bytes))
}


#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let matches = Command::new("Client")
        .about("Runs client")
        .arg(arg!(--address <SOCKET>).default_value("127.0.0.1:11111"))
        .get_matches();

    let socket_address = matches.get_one::<String>("address").ok_or_else(|| anyhow!("There is always a value."))?;

    info!("Starting client...");
    run_client(socket_address).await.context("Client stopped running because of an error.")?;
    info!("Exiting client!...");

    Ok(())
}
