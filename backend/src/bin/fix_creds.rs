use rusqlite::{params, Connection};
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use argon2::{Argon2, Params, Version};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use rand::{thread_rng, RngCore};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_path = "data/autostonks.db";
    let conn = Connection::open(db_path)?;
    
    let master_key = std::env::var("AUTO_STONKS_MASTER_KEY").unwrap_or_default();
    if master_key.is_empty() {
        println!("Error: AUTO_STONKS_MASTER_KEY not set in environment");
        return Ok(());
    }

    let api_key = std::env::var("NEW_ALPACA_API_KEY").unwrap_or_default();
    let api_secret = std::env::var("NEW_ALPACA_API_SECRET").unwrap_or_default();
    
    if api_key.is_empty() || api_secret.is_empty() {
        println!("Error: NEW_ALPACA_API_KEY or NEW_ALPACA_API_SECRET not set");
        return Ok(());
    }

    let mut stmt = conn.prepare("SELECT value FROM app_config WHERE key = 'credential_salt'")?;
    let salt: String = stmt.query_row([], |row| row.get(0))?;
    
    let argon2 = Argon2::new(
        argon2::Algorithm::Argon2id,
        Version::V0x13,
        Params::new(19456, 2, 1, Some(32)).unwrap(),
    );
    let mut key = [0u8; 32];
    argon2.hash_password_into(master_key.as_bytes(), salt.as_bytes(), &mut key).unwrap();
    let cipher = Aes256Gcm::new_from_slice(&key)?;

    // Encrypt Key
    let mut nonce_bytes = [0u8; 12];
    thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let encrypted_key = cipher.encrypt(nonce, api_key.as_bytes()).unwrap();
    let mut combined_key = nonce_bytes.to_vec();
    combined_key.extend_from_slice(&encrypted_key);
    let key_b64 = STANDARD.encode(combined_key);

    // Encrypt Secret
    let mut nonce_bytes = [0u8; 12];
    thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let encrypted_secret = cipher.encrypt(nonce, api_secret.as_bytes()).unwrap();
    let mut combined_secret = nonce_bytes.to_vec();
    combined_secret.extend_from_slice(&encrypted_secret);
    let secret_b64 = STANDARD.encode(combined_secret);

    // Update the first Alpaca credential found
    let mut stmt = conn.prepare("SELECT id FROM credentials WHERE provider = 'alpaca' LIMIT 1")?;
    let id: String = stmt.query_row([], |row| row.get(0))?;

    conn.execute(
        "UPDATE credentials SET api_key_encrypted = ?1, api_secret_encrypted = ?2 WHERE id = ?3",
        params![key_b64, secret_b64, id],
    )?;

    println!("Successfully updated credential {} with new keys", id);

    Ok(())
}
