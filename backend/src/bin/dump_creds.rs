use rusqlite::{params, Connection};
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use argon2::{Argon2, Params, Version};
use base64::{engine::general_purpose::STANDARD, Engine as _};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_path = "data/autostonks.db";
    let conn = Connection::open(db_path)?;
    
    let master_key = std::env::var("AUTO_STONKS_MASTER_KEY").unwrap_or_default();
    if master_key.is_empty() {
        println!("Error: AUTO_STONKS_MASTER_KEY not set in environment");
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

    let mut stmt = conn.prepare("SELECT id, label, api_key_encrypted FROM credentials")?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?))
    })?;

    println!("Stored Credentials:");
    for row in rows {
        let (id, label, encrypted) = row?;
        let decoded = STANDARD.decode(&encrypted).unwrap();
        let nonce = Nonce::from_slice(&decoded[..12]);
        let ciphertext = &decoded[12..];
        let decrypted_vec = cipher.decrypt(nonce, ciphertext).unwrap_or_else(|_| b"DECRYPTION FAILED".to_vec());
        let key_str = String::from_utf8_lossy(&decrypted_vec);
        println!(" - ID: {}, Label: {}, Key: {}", id, label, key_str);
    }

    Ok(())
}
