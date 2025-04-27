pub mod db;

pub mod password_hashing {
    use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
    use argon2::Argon2;
    use rand::rngs::OsRng;
    use anyhow::{Result, anyhow};


    /// Hash password using argon2 and return the hash.
    pub async fn hash_password(password: &String) -> Result<String> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = match argon2.hash_password(password.as_bytes(), &salt) {
            Ok(password_hash) => password_hash.to_string(),
            Err(e) => {
                return Err(anyhow!("Failed to hash password: {}", e));
            }
        };
        Ok(password_hash)
    }

    /// Verify a password against some hashed password.
    pub async fn verify_password(password: &String, password_hash: &String) -> Result<()> {
        let parsed_hash = match PasswordHash::new(password_hash) {
            Ok(parsed_hash) => parsed_hash,
            Err(e) => {
                return Err(anyhow!("Failed to parse hashed password: {}", e));
            }
        };
        match Argon2::default().verify_password(password.as_bytes(), &parsed_hash) {
            Ok(_) => Ok(()),
            Err(e) => {
                return Err(anyhow!("Failed to verify password: {}", e));
            }
        }
    }
}


pub mod http_server {
    use anyhow::Result;
    use axum::{extract::Path, http::StatusCode, response::Json, routing::{get, delete, get_service}, Extension, Router};
    use log::error;
    use sqlx::{Pool, Sqlite};
    use tower_http::services::fs::ServeFile;
    use tokio::net::TcpListener;

    use crate::db;

    /// Define routes and actions and run an http server.
    pub async fn run_http_server(http_socket_address: &str, connection_pool: Pool<Sqlite>, static_dir: &str) -> Result<()> {
        let app = Router::new()
            // Serve an html file to a client browser.
            .route(
                "/",
                get_service(
                    ServeFile::new(format!("{}/index.html", static_dir))
                )
            )
            // Get all messages sent by one specific user.
            .route(
                "/api/users/{id}/messages",
                get(get_messages)
            )
            // Get all users from database.
            .route(
                "/api/users",
                get(get_users)
            )
            // Remove a user from database (along with all messages sent by him).
            .route(
                "/api/users/{id}",
                delete(remove_user)
            )
            .layer(Extension(connection_pool));

        let listener = TcpListener::bind(http_socket_address).await.unwrap();
        axum::serve(listener, app).await.unwrap();

        Ok(())
    }

    /// Get all messages sent by a user with specified id.
    async fn get_messages(
        Path(id): Path<i64>,
        Extension(connection_pool): Extension<Pool<Sqlite>>
    ) -> Result<Json<Vec<String>>, StatusCode> {
        match db::get_messages_by_user(&connection_pool, &id).await {
            Ok(messages) => Ok(Json(messages)),
            Err(e) => {
                error!("Failed to get messages from database: {}", e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }

    /// Get all users from database.
    async fn get_users(
        Extension(connection_pool): Extension<Pool<Sqlite>>
    ) -> Result<Json<Vec<(i64, String)>>, StatusCode> {
        match db::get_all_users(&connection_pool).await {
            Ok(users) => Ok(Json(users)),
            Err(e) => {
                error!("Failed to get users from database: {}", e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }

    /// Remove a user from a database.
    async fn remove_user(
        Path(id): Path<i64>,
        Extension(connection_pool): Extension<Pool<Sqlite>>
    ) -> Result<(), StatusCode> {
        match db::delete_user(&connection_pool, &id).await {
            Ok(_) => Ok(()),
            Err(e) => {
                error!("Failed when removing user from database: {}", e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}