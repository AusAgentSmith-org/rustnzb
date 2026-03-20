use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

const ACCESS_TOKEN_TTL: Duration = Duration::from_secs(15 * 60); // 15 minutes
const REFRESH_TOKEN_TTL: Duration = Duration::from_secs(30 * 24 * 60 * 60); // 30 days

struct TokenEntry {
    expires_at: Instant,
}

pub struct TokenStore {
    access_tokens: RwLock<HashMap<String, TokenEntry>>,
    refresh_tokens: RwLock<HashMap<String, TokenEntry>>,
}

#[derive(Serialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: &'static str,
    pub expires_in: u64,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

fn generate_token() -> String {
    let bytes: [u8; 32] = rand::random();
    hex::encode(bytes)
}

impl TokenStore {
    pub fn new() -> Self {
        Self {
            access_tokens: RwLock::new(HashMap::new()),
            refresh_tokens: RwLock::new(HashMap::new()),
        }
    }

    pub fn create_tokens(&self) -> TokenResponse {
        let access_token = generate_token();
        let refresh_token = generate_token();
        let now = Instant::now();

        self.access_tokens.write().insert(
            access_token.clone(),
            TokenEntry {
                expires_at: now + ACCESS_TOKEN_TTL,
            },
        );
        self.refresh_tokens.write().insert(
            refresh_token.clone(),
            TokenEntry {
                expires_at: now + REFRESH_TOKEN_TTL,
            },
        );

        TokenResponse {
            access_token,
            refresh_token,
            token_type: "Bearer",
            expires_in: ACCESS_TOKEN_TTL.as_secs(),
        }
    }

    pub fn validate_access_token(&self, token: &str) -> bool {
        let tokens = self.access_tokens.read();
        tokens
            .get(token)
            .is_some_and(|entry| entry.expires_at > Instant::now())
    }

    pub fn refresh(&self, refresh_token: &str) -> Option<TokenResponse> {
        let valid = {
            let tokens = self.refresh_tokens.read();
            tokens
                .get(refresh_token)
                .is_some_and(|entry| entry.expires_at > Instant::now())
        };

        if !valid {
            return None;
        }

        // Revoke old refresh token (rotation)
        self.refresh_tokens.write().remove(refresh_token);
        Some(self.create_tokens())
    }

    pub fn revoke_refresh_token(&self, refresh_token: &str) {
        self.refresh_tokens.write().remove(refresh_token);
    }

    pub fn cleanup_expired(&self) {
        let now = Instant::now();
        self.access_tokens
            .write()
            .retain(|_, entry| entry.expires_at > now);
        self.refresh_tokens
            .write()
            .retain(|_, entry| entry.expires_at > now);
    }
}

// --- Credential Store ---

#[derive(Serialize, Deserialize, Clone)]
pub struct StoredCredentials {
    pub username: String,
    pub password: String,
}

pub struct CredentialStore {
    credentials: RwLock<Option<StoredCredentials>>,
    file_path: PathBuf,
}

impl CredentialStore {
    pub fn new(config_dir: PathBuf) -> Self {
        let file_path = config_dir.join("credentials.json");
        let credentials = if file_path.exists() {
            std::fs::read_to_string(&file_path)
                .ok()
                .and_then(|contents| serde_json::from_str(&contents).ok())
        } else {
            None
        };
        Self {
            credentials: RwLock::new(credentials),
            file_path,
        }
    }

    pub fn has_credentials(&self) -> bool {
        self.credentials.read().is_some()
    }

    pub fn set_credentials(&self, creds: StoredCredentials) -> Result<(), std::io::Error> {
        let json = serde_json::to_string_pretty(&creds)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        if let Some(parent) = self.file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&self.file_path, &json)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(
                &self.file_path,
                std::fs::Permissions::from_mode(0o600),
            )?;
        }
        *self.credentials.write() = Some(creds);
        Ok(())
    }

    pub fn validate(&self, username: &str, password: &str) -> bool {
        match &*self.credentials.read() {
            Some(creds) => {
                constant_time_eq(username.as_bytes(), creds.username.as_bytes())
                    && constant_time_eq(password.as_bytes(), creds.password.as_bytes())
            }
            None => false,
        }
    }
}

/// Constant-time byte comparison to prevent timing attacks.
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter()
        .zip(b.iter())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}
