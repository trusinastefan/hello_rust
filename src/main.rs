use std::{env, io, str};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use serde_cbor::{to_vec, from_slice};
use serde_derive::{Deserialize, Serialize};


#[derive(Serialize, Deserialize)]
enum MessageType {
    Text(String),
    Image(Vec<u8>),
    File(String, Vec<u8>),
}


fn run_server(socket_address: &str) -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind(socket_address)?;
    let clients: Arc<Mutex<HashMap<SocketAddr, TcpStream>>> = Arc::new(Mutex::new(HashMap::new()));
    for client_stream in listener.incoming() {
        let client_stream = client_stream?;
        let client_stream_cloned = client_stream.try_clone()?;
        let client_address = client_stream.peer_addr()?;
        let clients_cloned = Arc::clone(&clients);
        {
            match clients_cloned.lock() {
                Ok(mut lock) => {
                    lock.insert(client_address.clone(), client_stream);
                },
                Err(e) => {
                    return Err(format!("{}", e).into());
                }
            };
        }
        thread::spawn(move || {
            if let Err(e) = handle_client(client_address, client_stream_cloned, clients_cloned) {
                eprintln!("{}", e);
            };
        });
    }
    Ok(())
}


fn handle_client(client_address: SocketAddr, client_stream: TcpStream, clients: Arc<Mutex<HashMap<SocketAddr, TcpStream>>>) -> Result<(), Box<dyn Error>> {
    loop {
        let received_bytes = receive_bytes(&client_stream)?;
        match clients.lock() {
            Ok(lock) => {
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


fn receive_bytes(mut stream: &TcpStream) -> Result<Vec<u8>, io::Error> {
    let mut bytes_len_buf = [0u8; 4];
    stream.read_exact(&mut bytes_len_buf)?;
    let bytes_len = u32::from_be_bytes(bytes_len_buf) as usize;
    let mut buffer = vec![0u8; bytes_len];
    stream.read_exact(&mut buffer)?;
    Ok(buffer)
}


fn send_bytes(mut stream: &TcpStream, bytes: &[u8]) -> Result<(), Box<dyn Error>> {
    let len = bytes.len() as u32;
    stream.write(&len.to_be_bytes())?;
    stream.write_all(bytes)?;
    Ok(())
}


fn run_client(socket_address: &str) -> Result<(), Box<dyn Error>> {
    let continue_running: Arc<Mutex<bool>> = Arc::new(Mutex::new(true));
    let continue_running_cloned = Arc::clone(&continue_running);
    let stream = TcpStream::connect(socket_address)?;
    stream.set_read_timeout(Some(Duration::from_secs(1)))?;
    let stream_cloned = stream.try_clone()?;
    let handle = thread::spawn(move || {
        loop {
            match receive_bytes(&stream) {
                Ok(received_bytes) => {
                    if let Err(e) = handle_received_data_in_client(&received_bytes) {
                        eprintln!("{}", e);
                        break;
                    };
                },
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
        let bytes = prepare_data_based_on_user_input(user_input);
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


fn get_line_from_user() -> Result<String, Box<dyn Error>> {
    let mut input_str = String::new();
    io::stdin().read_line(&mut input_str)?;
    Ok(input_str)
}


fn handle_received_data_in_client(received_bytes: &Vec<u8>) -> Result<(), Box<dyn Error>> {
    let received_message = str::from_utf8(&received_bytes)?;
    println!("{}", received_message);
    Ok(())
}


fn prepare_data_based_on_user_input(user_input: String) -> Vec<u8> {
    user_input.into_bytes()
}


fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() == 1 {
        println!("Command not specified. Exiting program...");
    } else if args.len() == 2 && args[1] == "run-server" {
        println!("Starting server!");
        match run_server("127.0.0.1:7878") {
            Ok(()) => {
                println!("Exiting server!");
            },
            Err(e) => {
                eprintln!("{}", e);
            }
        };
    } else if args.len() == 2 && args[1] == "run-client" {
        println!("Starting client!");
        match run_client("127.0.0.1:7878") {
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
