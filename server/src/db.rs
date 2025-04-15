use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};
use anyhow::{Context, Result, anyhow};


/// Create a connection pool.
pub async fn create_connection_pool(database_url: &str) -> Result<SqlitePool> {
    let pool = SqlitePoolOptions::new()
        .max_connections(7)
        .connect(database_url)
        .await
        .context("Failed to create a pool.")?;
    Ok(pool)
}


/// Add a user into the 'users' table.
pub async fn add_user(pool: &SqlitePool, username: &str, password_hash: &str) -> Result<i64> {
    let rec = sqlx::query!(
        r#"
        INSERT INTO users (username, password_hash)
        VALUES (?, ?)
        RETURNING id
        "#,
        username,
        password_hash
    )
    .fetch_one(pool)
    .await
    .context("Failed to add new user into database.")?;
    
    Ok(rec.id)
}


/// Get a user entry from the 'users' table.
pub async fn get_user(pool: &SqlitePool, username: &str) -> Result<(i64, String)> {
    let rec = sqlx::query!(
        r#"
        SELECT id, password_hash
        FROM users
        WHERE username = ?
        "#,
        username
    )
    .fetch_one(pool)
    .await
    .context("Failed to get a user entry in a database")?;

    let id = rec.id.ok_or(anyhow!("The value of id not returned from database."))?;
    Ok((id, rec.password_hash))
}


/// Add a message into the messages table.
pub async fn add_message(pool: &SqlitePool, user_id: &i64, contents: &str) -> Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO messages (user_id, content)
        VALUES (?, ?)
        "#,
        user_id,
        contents
    )
    .execute(pool)
    .await
    .context("Failed to add message into database.")?;
    
    Ok(())
}
