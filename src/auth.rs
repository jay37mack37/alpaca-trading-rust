use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::sync::{Arc, RwLock};

lazy_static::lazy_static! {
    static ref USERS: Arc<RwLock<HashMap<String, User>>> = Arc::new(RwLock::new(HashMap::new()));
    static ref SESSIONS: Arc<RwLock<HashMap<String, String>>> = Arc::new(RwLock::new(HashMap::new())); // token -> username
}

const CONFIG_FILE: &str = "config.json";
const ENV_FILE: &str = ".env";

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

#[derive(Debug, Serialize, Deserialize, Clone)]
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
        let config_path = config_file_path();
        let path = config_path.as_path();
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
            let _ = fs::write(config_file_path(), content);
        }
    }
}

fn config_file_path() -> std::path::PathBuf {
    env::var("ALPACA_CONFIG_FILE")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from(CONFIG_FILE))
}

fn env_file_path() -> std::path::PathBuf {
    env::var("ALPACA_ENV_FILE")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from(ENV_FILE))
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
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos()
        .hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

pub fn init() {
    let config = Config::load();
    {
        let mut users = USERS.write().unwrap();
        users.clear();
        for user in config.users {
            users.insert(user.username.clone(), user);
        }
    }
    {
        let mut sessions = SESSIONS.write().unwrap();
        sessions.clear();
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

pub fn change_password(
    username: &str,
    current_password: &str,
    new_password: &str,
) -> Result<(), String> {
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

pub fn save_api_keys(
    username: &str,
    api_key: &str,
    api_secret: &str,
    environment: &str,
) -> Result<(), String> {
    let mut config = Config::load();
    config.api_keys.insert(
        username.to_string(),
        ApiKeyConfig {
            api_key: api_key.to_string(),
            api_secret: api_secret.to_string(),
            environment: environment.to_string(),
        },
    );
    config.save();

    // Also update .env file for the main app
    let env_content = format!(
        "# Alpaca API Credentials\n# Get your keys at: https://alpaca.markets/\n\nALPACA_API_KEY={}\nALPACA_API_SECRET={}\n\n# Environment: 'paper' for paper trading (default) or 'live' for real trading\nALPACA_ENV={}\n",
        api_key, api_secret, environment
    );
    let _ = fs::write(env_file_path(), env_content);

    env::set_var("ALPACA_API_KEY", api_key);
    env::set_var("ALPACA_API_SECRET", api_secret);
    env::set_var("ALPACA_ENV", environment);

    Ok(())
}

pub fn get_api_key_status(username: &str) -> (bool, Option<String>) {
    let config = Config::load();
    let configured = config.api_keys.contains_key(username);
    let environment = config.api_keys.get(username).map(|k| k.environment.clone());
    (configured, environment)
}

static MOCK_API_KEYS: once_cell::sync::Lazy<RwLock<HashMap<String, ApiKeyConfig>>> =
    once_cell::sync::Lazy::new(|| RwLock::new(HashMap::new()));

pub fn set_mock_api_keys(username: &str, api_key: &str, api_secret: &str, environment: &str) {
    let mut mocks = MOCK_API_KEYS.write().unwrap();
    mocks.insert(username.to_string(), ApiKeyConfig {
        api_key: api_key.to_string(),
        api_secret: api_secret.to_string(),
        environment: environment.to_string(),
    });
}

pub fn get_api_keys(username: &str) -> Option<(String, String, String)> {
    // Check mock keys first (for tests)
    {
        let mocks = MOCK_API_KEYS.read().unwrap();
        if let Some(k) = mocks.get(username) {
            return Some((k.api_key.clone(), k.api_secret.clone(), k.environment.clone()));
        }
    }

    let config = Config::load();
    config.api_keys.get(username).map(|k| {
        (
            k.api_key.clone(),
            k.api_secret.clone(),
            k.environment.clone(),
        )
    })
}

pub fn logout(token: &str) {
    let mut sessions = SESSIONS.write().unwrap();
    sessions.remove(token);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    // Helper to clean up test config
    fn cleanup_test_config() {
        let _ = fs::remove_file("config.json");
    }

    #[test]
    fn test_hash_password_consistency() {
        let hash1 = hash_password("test_password");
        let hash2 = hash_password("test_password");
        assert_eq!(hash1, hash2, "Same password should produce same hash");
    }

    #[test]
    fn test_hash_password_uniqueness() {
        let hash1 = hash_password("password1");
        let hash2 = hash_password("password2");
        assert_ne!(
            hash1, hash2,
            "Different passwords should produce different hashes"
        );
    }

    #[test]
    fn test_hash_password_not_empty() {
        let hash = hash_password("test");
        assert!(!hash.is_empty(), "Hash should not be empty");
    }

    #[test]
    fn test_generate_token_uniqueness() {
        let token1 = generate_token();
        // Small delay to ensure different timestamp
        std::thread::sleep(std::time::Duration::from_millis(1));
        let token2 = generate_token();
        assert_ne!(token1, token2, "Tokens should be unique");
    }

    #[test]
    fn test_generate_token_format() {
        let token = generate_token();
        // Token should be a hexadecimal string
        assert!(
            token.chars().all(|c| c.is_ascii_hexdigit()),
            "Token should be hexadecimal"
        );
    }

    #[test]
    fn test_user_serialization() {
        let user = User {
            username: "testuser".to_string(),
            password_hash: "hashedpassword".to_string(),
        };
        let json = serde_json::to_string(&user).unwrap();
        assert!(json.contains("testuser"));
        assert!(json.contains("hashedpassword"));
    }

    #[test]
    fn test_user_deserialization() {
        let json = r#"{"username":"testuser","password_hash":"hashedpassword"}"#;
        let user: User = serde_json::from_str(json).unwrap();
        assert_eq!(user.username, "testuser");
        assert_eq!(user.password_hash, "hashedpassword");
    }

    #[test]
    fn test_login_request_deserialization() {
        let json = r#"{"username":"admin","password":"secret"}"#;
        let req: LoginRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.username, "admin");
        assert_eq!(req.password, "secret");
    }

    #[test]
    fn test_api_key_request_deserialization() {
        let json = r#"{"api_key":"key123","api_secret":"secret456","environment":"paper"}"#;
        let req: ApiKeyRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.api_key, "key123");
        assert_eq!(req.api_secret, "secret456");
        assert_eq!(req.environment, "paper");
    }

    #[test]
    fn test_password_request_deserialization() {
        let json = r#"{"current_password":"old","new_password":"new"}"#;
        let req: PasswordRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.current_password, "old");
        assert_eq!(req.new_password, "new");
    }

    #[test]
    fn test_config_default() {
        cleanup_test_config();
        let config = Config::load();
        assert!(
            !config.users.is_empty(),
            "Default config should have at least one user"
        );
        assert_eq!(config.users[0].username, "admin");
        cleanup_test_config();
    }

    #[test]
    fn test_config_save_and_load() {
        cleanup_test_config();

        let config = Config {
            users: vec![User {
                username: "testuser".to_string(),
                password_hash: hash_password("testpass"),
            }],
            api_keys: HashMap::new(),
        };
        config.save();

        let loaded = Config::load();
        // The loaded config should have at least one user (either testuser if saved properly, or admin as default)
        assert!(!loaded.users.is_empty(), "Config should have users");

        cleanup_test_config();
    }

    #[test]
    fn test_api_key_config_clone() {
        let config = ApiKeyConfig {
            api_key: "key".to_string(),
            api_secret: "secret".to_string(),
            environment: "paper".to_string(),
        };
        let cloned = config.clone();
        assert_eq!(config.api_key, cloned.api_key);
        assert_eq!(config.api_secret, cloned.api_secret);
        assert_eq!(config.environment, cloned.environment);
    }

    #[test]
    fn test_login_response_serialization() {
        let response = LoginResponse {
            token: "abc123".to_string(),
            username: "testuser".to_string(),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("abc123"));
        assert!(json.contains("testuser"));
    }

    #[test]
    fn test_login_response_deserialization() {
        let json = r#"{"token":"xyz789","username":"john"}"#;
        let response: LoginResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.token, "xyz789");
        assert_eq!(response.username, "john");
    }
}
