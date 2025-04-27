use anyhow::{anyhow, Context, Result};
use clap::{Arg, Command};
use log::{error, info};
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpListener;
use tokio::sync::Mutex;

use server::password_hashing::{hash_password, verify_password};
use server::http_server::run_http_server;
use shared::{receive_message, send_message, MessageType};
use server::db;

type SharedWriteHalf = Arc<Mutex<OwnedWriteHalf>>;

/// This function runs server.
/// It listens for connections from clients in a loop.
/// Each time a client connects, a new async task is spawned that handles that connection.
async fn run_server(socket_address: &str, connection_pool: SqlitePool) -> Result<()> {
    let listener = TcpListener::bind(socket_address)
        .await
        .context("TcpListener failed to bind to a socket address.")?;
    let client_writers: Arc<Mutex<HashMap<SocketAddr, SharedWriteHalf>>> =
        Arc::new(Mutex::new(HashMap::new()));

    loop {
        // Create a new stream for each incomming connection.
        let (client_stream, client_address) = listener
            .accept()
            .await
            .context("Failed to accept a new connection from a client.")?;
        // Split each stream into a reader and a writer.
        let (client_reader, client_writer) = client_stream.into_split();

        // Add writer to respective hash maps. The socket address is key.
        {
            let mut lock = client_writers.lock().await;
            lock.insert(client_address.clone(), Arc::new(Mutex::new(client_writer)));
        }

        // Clone reader hash map.
        let client_writers_cloned = Arc::clone(&client_writers);
        // Clone connection pool.
        let connection_pool_cloned = connection_pool.clone();
        // For each incomming connection, there is a separate async task.
        tokio::spawn(async move {
            let client_address_for_removal = client_address.clone();
            let client_writers_for_removal = Arc::clone(&client_writers_cloned);

            // Start client handler that receives and forwards messages.
            if let Err(e) = handle_client(
                client_address,
                client_reader,
                client_writers_cloned,
                connection_pool_cloned,
            )
            .await
            {
                error!(
                    "Client handler on server stopped executing due to an error: {}",
                    e
                );
            };

            // After a spawned tasks comes to an end, remove writer associated with the corresponding client.
            remove_client_writer(client_address_for_removal, client_writers_for_removal).await;
        });
    }
}

/// This function is executed as a separate async task for each incomming connection.
/// First, it handles user authentication.
/// Then a loop follows where the server waits for incomming messages.
/// If a message arrives, it is saved into a database and resent to all other clients.
async fn handle_client(
    client_address: SocketAddr,
    mut client_reader: OwnedReadHalf,
    client_writers: Arc<Mutex<HashMap<SocketAddr, SharedWriteHalf>>>,
    connection_pool: SqlitePool,
) -> Result<()> {
    // Try to authenticate user. If not successful, the connection will be dropped.
    let (user_id, _username) = match authenticate_user(
        &mut client_reader,
        &client_address,
        &client_writers,
        &connection_pool,
    )
    .await
    {
        Some((id, name)) => (id, name),
        None => {
            return Ok(());
        }
    };
    loop {
        // Wait for data from a client.
        //let received_bytes = receive_bytes(&mut client_reader).await.context("Failed when receiving bytes.")?;
        let received_message = receive_message(&mut client_reader)
            .await
            .context("Failed when receiving a message.")?;

        // Save received message in a database.
        save_message_in_database(&connection_pool, &user_id, &received_message)
            .await
            .context("Failed to save message in a database.")?;

        // Send received data to all clients except the one from which the data were received.
        let lock = client_writers.lock().await;
        for address in lock.keys() {
            if *address != client_address {
                let shared_writer = lock
                    .get(address)
                    .ok_or_else(|| anyhow!("Address not found in HashMap."))?;
                let mut lock_writer = shared_writer.lock().await;
                if let Err(e) = send_message(&mut *lock_writer, &received_message).await {
                    error!("Failed when sending bytes to address {}: {}", *address, e);
                }
            }
        }
    }
}

/// Go through the whole process of authentification, including communication with a database.
async fn authenticate_user(
    reader: &mut OwnedReadHalf,
    client_address: &SocketAddr,
    client_writers: &Arc<Mutex<HashMap<SocketAddr, SharedWriteHalf>>>,
    connection_pool: &SqlitePool,
) -> Option<(i64, String)> {
    // Wait for authentication request message.
    let (action, username, password) = match receive_message(reader).await {
        // Data received and passed to the handler.
        Ok(MessageType::AuthRequest(action, username, password)) => {
            info!("Received authentication request from {}.", &username);
            (action, username, password)
        }

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

    // Authenticate and return success status, message that should be sent to client and user id.
    let (user_id, message_from_server) =
        handle_auth_request(connection_pool, &action, &username, &password).await;

    // Send authentication response message back to the user.
    let lock = client_writers.lock().await;
    let shared_writer = match lock.get(client_address) {
        Some(w) => w,
        None => {
            error!("Address not found in HashMap.");
            return None;
        }
    };
    let mut lock_writer = shared_writer.lock().await;

    match user_id {
        // If id was returned, that means that the user was authented.
        Some(id) => {
            info!("Authentication succeeded. Sending response back to user.");
            let auth_response_message = MessageType::AuthResponse(true, message_from_server);
            // Send auth response confirming that the user was authenticated.
            match send_message(&mut *lock_writer, &auth_response_message).await {
                Ok(_) => {
                    return Some((id, username));
                }
                Err(e) => {
                    error!("Error while sending authentication response: {}", e);
                    return None;
                }
            }
        }
        // If no id was returned, the user was not authented.
        None => {
            info!("Authentication did not succeed. Sending response back to user.");
            let auth_response_message = MessageType::AuthResponse(false, message_from_server);
            // Send auth response informing client that the user was not authenticated.
            match send_message(&mut *lock_writer, &auth_response_message).await {
                Ok(_) => {
                    return None;
                }
                Err(e) => {
                    error!("Error while sending authentication response: {}", e);
                    return None;
                }
            }
        }
    }
}

/// Based on parameters, try to either register or authenticate user. Produce a response message for client.
async fn handle_auth_request(
    connection_pool: &SqlitePool,
    action: &String,
    username: &String,
    password: &String,
) -> (Option<i64>, String) {
    if action == "R" {
        return register(connection_pool, username, password).await;
    } else if action == "L" {
        return login(connection_pool, username, password).await;
    } else {
        return (
            None,
            "Authentication failed because of incorrect action identifier. (Must be 'R or 'L'')"
                .to_string(),
        );
    }
}

/// Register a user.
async fn register(
    connection_pool: &SqlitePool,
    username: &String,
    password: &String,
) -> (Option<i64>, String) {
    let password_hash = match hash_password(password).await {
        Ok(password_hash) => password_hash,
        Err(e) => {
            error!("Failed to hash password: {}", e);
            return (
                None,
                "Registration not successful. Try a different password.".to_string(),
            );
        }
    };
    match db::add_user(connection_pool, username, &password_hash).await {
        Ok(user_id) => {
            info!("Successful registration of a user.");
            return (Some(user_id), "Registration successful.".to_string());
        }
        Err(e) => {
            info!("Failed to register user: {}", e);
            return (
                None,
                "Registration not successful. Try a different username.".to_string(),
            );
        }
    }
}

/// Log in a user.
/// First it is checked if the user has an entry in database and user id and password hash are obtained.
/// Then, the password provided by the user is verified against the password hash from the database.
async fn login(
    connection_pool: &SqlitePool,
    username: &String,
    password: &String,
) -> (Option<i64>, String) {
    let (user_id, password_hash) = match db::get_user(connection_pool, username).await {
        Ok((user_id, password_hash)) => (user_id, password_hash),
        Err(e) => {
            info!("Login not successful: {}", e);
            return (None, "Login not successful.".to_string());
        }
    };
    match verify_password(password, &password_hash).await {
        Ok(_) => {
            info!("Login successful.");
            return (Some(user_id), "Successfully logged in.".to_string());
        }
        Err(e) => {
            info!("Login not successful: {}", e);
            return (
                None,
                "Login not successful. The password seems to be incorrect.".to_string(),
            );
        }
    }
}

/// Take a message and save it into a database.
/// Each message is associated with its author.
async fn save_message_in_database(
    connection_pool: &SqlitePool,
    user_id: &i64,
    message: &MessageType,
) -> Result<()> {
    let contents = match message {
        MessageType::Text(text) => text.clone(),
        MessageType::Image(_) => "SENT IMAGE".to_string(),
        MessageType::File(name, _) => format!("FILE SENT: {}", name),
        _ => {
            return Err(anyhow!("This message type cannot be saved in database."));
        }
    };
    db::add_message(connection_pool, user_id, &contents)
        .await
        .context("Failed to save message in a database")?;

    Ok(())
}

/// Remove an invalid writer from a HashMap.
async fn remove_client_writer(
    client_address: SocketAddr,
    client_writers: Arc<Mutex<HashMap<SocketAddr, SharedWriteHalf>>>,
) -> () {
    let mut lock = client_writers.lock().await;
    match lock.remove(&client_address) {
        Some(_) => {
            info!(
                "Removing writer associated with socket {} from HashMap.",
                &client_address
            );
        }
        None => {
            error!(
                "Writer associated with socket {} not found in HashMap.",
                &client_address
            );
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    // Process command line arguments.
    let matches = Command::new("Server")
        .about("Runs server")
        .arg(
            Arg::new("chat-socket")
            .short('c')
            .long("chat-socket")
            .value_name("CHAT_SOCKET")
            .default_value("0.0.0.0:11111")
            .help("Socket on which the chat server should listen for incomming client connections.")
        )
        .arg(
            Arg::new("http-socket")
            .short('w')
            .long("http-socket")
            .value_name("HTTP_SOCKET")
            .default_value("0.0.0.0:80")
            .help("HTTP socket through which chat server admin page can be accessed.")
        )
        .arg(
            Arg::new("db-file")
            .short('d')
            .long("db-file")
            .value_name("DB_FILE")
            .default_value("server/chat_app_data.db")
            .help("Path to a '.db' file containing chat server sqlite database.")
        )
        .arg(
            Arg::new("static-dir")
            .short('s')
            .long("static-dir")
            .value_name("STATIC_DIR")
            .default_value("server/static")
            .help("Directory containing 'index.html' file.")
        )
        .get_matches();
    let chat_socket_address = matches
        .get_one::<String>("chat-socket")
        .ok_or_else(|| anyhow!("There is always a value."))?
        .clone();
    let http_socket_address = matches
        .get_one::<String>("http-socket")
        .ok_or_else(|| anyhow!("There is always a value."))?
        .clone();
    let db_file = matches
        .get_one::<String>("db-file")
        .ok_or_else(|| anyhow!("There is always a value."))?
        .clone();
    let static_dir = matches
        .get_one::<String>("static-dir")
        .ok_or_else(|| anyhow!("There is always a value."))?
        .clone();

    // Create a database connection pool.
    let database_url = format!("sqlite://{}", db_file);
    let connection_pool_http_server = db::create_connection_pool(&database_url)
        .await
        .context("Failed to create connection pool.")?;
    let connection_pool_chat_server = connection_pool_http_server.clone();

    // Run http server.
    let http_task = tokio::spawn(
        async move {
            info!("Starting http server...");
            if let Err(e) = run_http_server(&http_socket_address, connection_pool_http_server, &static_dir).await {
                    error!("HTTP server failed: {}", e);
                };
            info!("Exiting http server...");
        }
    );
    
    // Run chat server.
    let chat_task = tokio::spawn(async move {
        info!("Starting chat server...");
        if let Err(e) = run_server(&chat_socket_address, connection_pool_chat_server).await {
            error!("Chat server failed: {}", e);
        };
        info!("Exiting chat server...");
    });
    
    tokio::try_join!(http_task, chat_task)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use tokio::net::TcpStream;

    use super::*;

    #[tokio::test]
    async fn test_remove_client_writer() {
        let writers_to_clients: Arc<Mutex<HashMap<SocketAddr, SharedWriteHalf>>> =
        Arc::new(Mutex::new(HashMap::new()));

        let server_socket_address = "127.0.0.1:33333";
        let server_listener = TcpListener::bind(server_socket_address).await.unwrap();
        TcpStream::connect(server_socket_address).await.unwrap();
        let (server_stream, server_socket_address) = server_listener
            .accept()
            .await
            .unwrap();
        let (_, writer) = server_stream.into_split();
        
        {
            let mut lock = writers_to_clients.lock().await;
            lock.insert(server_socket_address, Arc::new(Mutex::new(writer)));
            assert_eq!(lock.len(), 1);
        }

        let cloned_writers_to_clients = writers_to_clients.clone();
        remove_client_writer(server_socket_address, cloned_writers_to_clients).await;
        {
            let lock = writers_to_clients.lock().await;
            assert_eq!(lock.len(), 0);
        }
    }
}
