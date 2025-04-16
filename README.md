RUST CHAT APP PROJECT  

DESCRIPTION  
This is a basic chat application project written in Rust. It contains code for both server and client. The project was implemented for the Rust programming course organised by Robot Dreams and Braiins.

STRUCTURE  
Root workspace contains three crates: client, server and shared. It has the following structure:

```
hello_rust/
├── Cargo.toml                                  # Workspace manifest
├── Cargo.lock
├── README.md
├── client/                                     # Client binary crate
│   ├── src/main.rs
│   └── Cargo.toml
├── server/                                     # Server binary crate
│   ├── .sqlx/                                  # Directory with sqlx query metadata .json files
│   ├── migrations/001_create_tables.sql        # A file specifying sqlite database structure
│   ├── src/
│   │   ├── db.rs                               # File with functions for database communication
│   │   └── main.rs
│   └── Cargo.toml
└── shared/                                     # Shared library crate
    ├── src/lib.rs
    └── Cargo.toml
```

The client binary crate contains a main function that, if called, starts a client app.
The server binary crate contains a main function that, if called, starts a server.
Both of these crates use functionalities implemented in the shared library crate.

BUILDING THE PROJECT  
You can download and build the project by following these instructions:

```
git clone https://github.com/trusinastefan/hello_rust.git
cd hello_rust
```
Create empty file named 'chat_app_data.db' in the 'server' directory.
```
cargo install sqlx-cli --no-default-features --features sqlite
cargo sqlx migrate run --database-url "sqlite://chat_app_data.db"
set SQLX_OFFLINE=1
cargo build --release
```

RUNNING SERVER  
Before any client instances are started, a server needs to be running and listening on a socket. The server can be started by running the following command:

```
cargo run -p server -- --address <socket_address>
```

You do not have to specify socket_address. In that case, a default loopback address is used, namely "127.0.0.1:11111". The socket_address specifies on which socket the server should be listening for connections.

RUNNING CLIENT  
Clients can be run if server is listening for connections. A client instance can be started by running the following command:

```
cargo run -p client -- --address <socket_address>
```

You do not have to specify socket_address. In that case, a default loopback address is used, namely "127.0.0.1:11111". The socket_address must be the same address as the one on which the server is listening.
After a client is started, user is prompted to choose if he wants to login or register and then to type his username and password. Registered users passwords are saved in database. If a user tries to register with a username that already exists, client will exit. If a user tries to login with incorrect username or password, client will exit.

USING THE CHAT APPLICATION  
To use the app, at least two clients should be connected to server. After a client app is started, it is waiting for user commands. There are four types of commands:

1. ".file <path>" command: If a user input starts with ".file ", it is supposed that the rest of the input represents a path to a file. If it is indeed a valid path, the file is sent to all other connected clients and saved to directory "./files". This directory must already exist.

2. ".image <path>" command: If a user input starts with ".image ", it is supposed that the rest of the input represents a path to a png image file. If this is the case, the file is sent to all other connected clients and saved to directory "./images". This directory must already exist.

3. ".quit" command: This command stops the client and exits.

4. All other strings will be sent as strings to all other connected clients printed in their console.
