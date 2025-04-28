use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};
use anyhow::{Context, Result, anyhow};


/// Create a connection pool and return it from the function.
/// This pool is used by functions executing database queries.
pub async fn create_connection_pool(database_url: &str) -> Result<SqlitePool> {
    let pool = SqlitePoolOptions::new()
        .max_connections(7)
        .connect(database_url)
        .await
        .context("Failed to create a pool.")?;
    Ok(pool)
}


/// Add a user entry into the 'users' table.
/// A new entry can be created by inserting username and a hashed password into the users table.
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
/// The two values we want to obtain in this manner are id and password hash of a user.
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
/// Each message is associated to its auther by using user id.
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


pub async fn get_messages_by_user(pool: &SqlitePool, user_id: &i64) -> Result<Vec<String>> {
    let rec= sqlx::query!(
        r#"
        SELECT content
        FROM messages
        WHERE user_id = ?
        "#,
        user_id
    )
    .fetch_all(pool)
    .await
    .context("Failed to get messages.")?;
    
    let messages: Vec<String> = rec.into_iter().map(|row| row.content).collect();
    Ok(messages)
}


pub async fn delete_messages_by_user(pool: &SqlitePool, user_id: &i64) -> Result<()> {
    sqlx::query!(
        r#"
        DELETE FROM messages
        WHERE user_id = ?
        "#,
        user_id
    )
    .execute(pool)
    .await
    .context("Failed to delete messages.")?;
    
    Ok(())
}


pub async fn delete_user(pool: &SqlitePool, user_id: &i64) -> Result<()> {
    sqlx::query!(
        r#"
        DELETE FROM users
        WHERE id = ?
        "#,
        user_id
    )
    .execute(pool)
    .await
    .context("Failed to delete.")?;
    
    Ok(())
}


pub async fn get_all_users(pool: &SqlitePool) -> Result<Vec<(i64, String)>> {
    let rec= sqlx::query!(
        r#"
        SELECT id, username
        FROM users
        "#
    )
    .fetch_all(pool)
    .await
    .context("Failed to get all users.")?;
    
    let users: Vec<(i64, String)> = rec.into_iter().map(
        |row| {
            let id = row.id.ok_or(anyhow!("A user entry has id null."))?;
            Ok((id, row.username))
        }
    ).collect::<Result<Vec<(i64, String)>>>().context("Something wrong with extracting data from users table.")?;
    Ok(users)
}
