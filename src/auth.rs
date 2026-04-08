use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::fs;
use std::path::Path;

lazy_static::lazy_static! {
    static ref USERS: Arc<RwLock<HashMap<String, User>>> = Arc::new(RwLock::new(HashMap::new()));
    static ref SESSIONS: Arc<RwLock<HashMap<String, String>>> = Arc::new(RwLock::new(HashMap::new())); // token -> username
}

const CONFIG_FILE: &str = "config.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub username: String,
    pub password_hash: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginResponse {
    pub token: String,
    pub username: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiKeyRequest {
    pub api_key: String,
    pub api_secret: String,
    pub environment: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub users: Vec<User>,
    pub api_keys: HashMap<String, ApiKeyConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ApiKeyConfig {
    pub api_key: String,
    pub api_secret: String,
    pub environment: String,
}

impl Config {
    pub fn load() -> Self {
        let path = Path::new(CONFIG_FILE);
        if path.exists() {
            if let Ok(content) = fs::read_to_string(path) {
                if let Ok(config) = serde_json::from_str(&content) {
                    return config;
                }
            }
        }
        // Default config with admin user
        let default = Config {
            users: vec![User {
                username: "admin".to_string(),
                password_hash: hash_password("admin123"),
            }],
            api_keys: HashMap::new(),
        };
        default.save();
        default
    }

    pub fn save(&self) {
        if let Ok(content) = serde_json::to_string_pretty(self) {
            let _ = fs::write(CONFIG_FILE, content);
        }
    }
}

fn hash_password(password: &str) -> String {
    // Simple hash for demo - in production use bcrypt or argon2
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    password.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

fn generate_token() -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::time::{SystemTime, UNIX_EPOCH};
    let mut hasher = DefaultHasher::new();
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos().hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

pub fn init() {
    let config = Config::load();
    let mut users = USERS.write().unwrap();
    for user in config.users {
        users.insert(user.username.clone(), user);
    }
}

pub fn login(username: &str, password: &str) -> Option<LoginResponse> {
    let users = USERS.read().unwrap();
    let user = users.get(username)?;

    if user.password_hash != hash_password(password) {
        return None;
    }

    let token = generate_token();
    {
        let mut sessions = SESSIONS.write().unwrap();
        sessions.insert(token.clone(), username.to_string());
    }

    Some(LoginResponse {
        token,
        username: username.to_string(),
    })
}

pub fn verify_token(token: &str) -> Option<String> {
    let sessions = SESSIONS.read().unwrap();
    sessions.get(token).cloned()
}

pub fn change_password(username: &str, current_password: &str, new_password: &str) -> Result<(), String> {
    let mut users = USERS.write().unwrap();
    let user = users.get(username).ok_or("User not found")?;

    if user.password_hash != hash_password(current_password) {
        return Err("Current password is incorrect".to_string());
    }

    let new_hash = hash_password(new_password);
    users.get_mut(username).unwrap().password_hash = new_hash;

    // Save to config
    let config = Config::load();
    let mut updated = config;
    for user in &mut updated.users {
        if user.username == username {
            user.password_hash = hash_password(new_password);
        }
    }
    updated.save();

    Ok(())
}

pub fn save_api_keys(username: &str, api_key: &str, api_secret: &str, environment: &str) -> Result<(), String> {
    let mut config = Config::load();
    config.api_keys.insert(username.to_string(), ApiKeyConfig {
        api_key: api_key.to_string(),
        api_secret: api_secret.to_string(),
        environment: environment.to_string(),
    });
    config.save();

    // Also update .env file for the main app
    let env_content = format!(
        "# Alpaca API Credentials\n# Get your keys at: https://alpaca.markets/\n\nALPACA_API_KEY={}\nALPACA_API_SECRET={}\n\n# Environment: 'paper' for paper trading (default) or 'live' for real trading\nALPACA_ENV={}\n",
        api_key, api_secret, environment
    );
    let _ = fs::write(".env", env_content);

    Ok(())
}

pub fn get_api_key_status(username: &str) -> (bool, Option<String>) {
    let config = Config::load();
    let configured = config.api_keys.contains_key(username);
    let environment = config.api_keys.get(username).map(|k| k.environment.clone());
    (configured, environment)
}

pub fn get_api_keys(username: &str) -> Option<(String, String, String)> {
    let config = Config::load();
    config.api_keys.get(username).map(|k| {
        (k.api_key.clone(), k.api_secret.clone(), k.environment.clone())
    })
}

pub fn logout(token: &str) {
    let mut sessions = SESSIONS.write().unwrap();
    sessions.remove(token);
}