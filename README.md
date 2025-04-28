# RUST CHAT APP PROJECT  

### DESCRIPTION  
This is a basic chat application project written in Rust. It contains code for both server and client. The project was implemented for the Rust programming course organised by Robot Dreams and Braiins.

### STRUCTURE  
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
│   │   ├── lib.rs                              # Library of functions for server crate.
│   │   └── main.rs
│   ├── static/index.html                       # File containing code for admin page.
│   ├── tests/integration.rs                    # File containing server crate integration tests.
│   └── Cargo.toml
└── shared/                                     # Shared library crate
    ├── src/lib.rs
    ├── tests/integration.rs                    # File containing shared crate integration tests.
    └── Cargo.toml
```

The client binary crate contains a main function that, if called, starts a client app.

The server binary crate contains a main function that, if called, starts a server.
In directory `.sqlx`, there are json files with compiled database queries. These files are used to check the queries at compile time.
The `migrations` directory contains a file defining database structure. This file can be used to build the database.
The `static` directory contains file `index.html` that contains code of the admin page.
The `tests` directory contains server crate integration tests.

Both of these crates use functionalities implemented in the shared library crate.
The `tests` directory contains shared crate integration tests.

### BUILDING THE PROJECT  
You can download and build the project by following these instructions:

```
git clone https://github.com/trusinastefan/hello_rust.git
```

Change current working directory to server:

```
cd hello_rust/server
```

Create empty file named `chat_app_data.db` in the `server` directory.

```
cargo install sqlx-cli --no-default-features --features sqlite
cargo sqlx migrate run --database-url "sqlite://chat_app_data.db"
```

Change current working directory to the project's root:

```
cd ..
```

Build the project:

```
cargo build --release
```

### RUNNING SERVER  
Before any client instances are started, a server needs to be running and listening on a socket. The server can be started by running the following command from the project root:

```
cargo run -p server -- --chat-socket <CHAT_SOCKET> --http-socket <HTTP_SOCKET> --db-file <DB_FILE> --static-dir <STATIC_DIR>
```

The application can also be run without using cargo by replacing `cargo run -p server --` through path to executable file.
All flags are optional and have a default value. The default values should allow the server to run if the project's root directory is also current working directory.
The `--chat-socket` flag specifies on which socket the chat server should be listening for client connections. The default value is `0.0.0.0:11111`.
The `--http-socket` flag specifies through which socket an admin page can be accessed. The default value is `0.0.0.0:80`. The `index.html` file is served through this socket.
The `--db-file` flag specifies the `.db` file containing sqlite database. The default value is `server/chat_app_data.db`.
The `--static-dir` flag specifies the `static` directory which contains file `index.html`. The default value is `server/static`.

### RUNNING CLIENT  
Clients can be run if server is listening for connections. A client instance can be started by running the following command:

```
cargo run -p client -- --chat-socket <CHAT_SOCKET>
```

The application can also be run without using cargo by replacing `cargo run -p client --` through path to executable file.
The value of the `--chat-socket` flag must be the same socket address as the one on which the server is listening. This flag is required.
After a client is started, user is prompted to choose if he wants to login or register and then to type his username and password. Registered user passwords are saved in database. If a user tries to register with a username that already exists, client will exit. If a user tries to login with incorrect username or password, client will exit.

### USING THE CHAT APPLICATION  
To use the app and see how it works, at least two clients should be connected to server. After a client app is started, it is waiting for user commands. There are four types of commands:

1. `.file <path>` command: If a user input starts with `.file `, it is supposed that the rest of the input represents a path to a file. If it is indeed a valid path, the file is sent to all other connected clients and saved into directory `./files`. This directory must already exist.

2. `.image <path>` command: If a user input starts with `.image `, it is supposed that the rest of the input represents a path to a png image file. If this is the case, the file is sent to all other connected clients and saved into directory `./images`. This directory must already exist.

3. `.quit` command: This command stops the client and exits.

4. All other strings will be sent as strings to all other connected clients and printed in their console.

### SERVER ADMIN PAGE  
When server is started, in addition to its other functionalities, it also runs an HTTP server at port 80. This HTTP server serves an admin page. On this page, it is possible to select from existing users and either delete that user or show all messages sent by that user. The messages are taken from database and the user deletion removes a user and all associated messages from the database.

### TESTING  
All tests can be executed by running the following command from the project root:

```
cargo test
```

### PROMETHEUS INTEGRATION  
The application provides `/metrics` endpoint on port 80 through which Prometheus can obtain collected metrics. There are two metrics provided:

1. `messages_counter`: This metric counts the number of messages sent through the server.

2. `active_connections_gauge`: This metric represents the number of currently active client connections.

### LOGGING  
Both client and server parts of this project use a logging library and provide `info` and `error` log messages. To see these messages in console, set the `RUST_LOG` environment variable to `info`. On Windows, this would be:

```
$Env:RUST_LOG = "info"
```
