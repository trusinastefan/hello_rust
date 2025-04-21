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
