pub mod db;

pub mod password_hashing {
    use anyhow::{anyhow, Result};
    use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
    use argon2::Argon2;
    use rand::rngs::OsRng;

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
    use axum::{
        extract::Path,
        http::{header::CONTENT_TYPE, HeaderMap, HeaderValue, StatusCode},
        response::{IntoResponse, Json},
        routing::{delete, get, get_service},
        Extension, Router,
    };
    use log::error;
    use prometheus::{Registry, Encoder, TextEncoder};
    use sqlx::{Pool, Sqlite};
    use tokio::net::TcpListener;
    use tower_http::services::fs::ServeFile;

    use crate::db;

    /// Define routes and actions and run an http server.
    pub async fn run_http_server(
        http_socket_address: &str,
        connection_pool: Pool<Sqlite>,
        static_dir: &str,
        registry: Registry
    ) -> Result<()> {
        let app = Router::new()
            // Serve an html file to a client browser.
            .route(
                "/",
                get_service(ServeFile::new(format!("{}/index.html", static_dir))),
            )
            // Get all messages sent by one specific user.
            .route("/api/users/{id}/messages", get(get_messages))
            // Get all users from database.
            .route("/api/users", get(get_users))
            // Remove a user from database (along with all messages sent by him).
            .route("/api/users/{id}", delete(remove_user))
            // Expose an endpoint for prometheus metrics.
            .route("/metrics", get(get(get_metrics)))
            .layer(Extension(connection_pool))
            .layer(Extension(registry));

        let listener = TcpListener::bind(http_socket_address).await.unwrap();
        axum::serve(listener, app).await.unwrap();

        Ok(())
    }

    /// Get all messages sent by a user with specified id.
    async fn get_messages(
        Path(id): Path<i64>,
        Extension(connection_pool): Extension<Pool<Sqlite>>,
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
        Extension(connection_pool): Extension<Pool<Sqlite>>,
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
        Extension(connection_pool): Extension<Pool<Sqlite>>,
    ) -> Result<(), StatusCode> {
        match db::delete_user(&connection_pool, &id).await {
            Ok(_) => Ok(()),
            Err(e) => {
                error!("Failed when removing user from database: {}", e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }

    // Get collected prometheus metrics.
    async fn get_metrics(
        Extension(registry): Extension<Registry>
    ) -> Result<impl IntoResponse, StatusCode> {
        let mut buffer = vec![];
        let encoder = TextEncoder::new();
        let metric_families = registry.gather();
        
        if let Err(err) = encoder.encode(&metric_families, &mut buffer) {
            error!("Failed to extract collected metrics into a buffer: {}", err);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }

        let mut headers = HeaderMap::new();
        let header_value = match HeaderValue::from_str(encoder.format_type()) {
            Ok(header_value) => header_value,
            Err(err) => {
                error!("Failed to create headers: {}", err);
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        };
        headers.insert(CONTENT_TYPE, header_value);

        Ok((StatusCode::OK, headers, buffer))
    }
}

pub mod metrics {
    use anyhow::{Context, Result};
    use prometheus::{Counter, Gauge, Opts};

    /// Create a metric that tracks the number of messages sent through the server by clients.
    pub async fn get_messages_counter() -> Result<Counter> {
        let messages_counter_opts = Opts::new(
            "messages_counter",
            "A counter for tracking the number of messages sent through the server",
        );
        let messages_counter = Counter::with_opts(messages_counter_opts)
            .context("Failed to create message counter metric.")?;
        Ok(messages_counter)
    }

    /// Create a metric that tracks the number of active connections to the server.
    pub async fn get_active_connections_gauge() -> Result<Gauge> {
        let active_connections_gauge_opts = Opts::new(
            "active_connections_gauge",
            "A gauge for tracking the number of active connections to the server",
        );
        let active_connections_gauge = Gauge::with_opts(active_connections_gauge_opts)
            .context("Failed to create active connections gauge metric.")?;
        Ok(active_connections_gauge)
    }
}
