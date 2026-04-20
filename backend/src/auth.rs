use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use axum::{
    extract::{Query, Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
};
use rand::RngCore;
use serde::Deserialize;
use subtle::ConstantTimeEq;
use tracing::info;

#[cfg(unix)]
use tracing::warn;

use crate::error::{AppError, AppResult};

/// Holds the API token that gates all `/api/*` endpoints (except `/api/health`).
///
/// The token is loaded from the `AUTO_STONKS_API_TOKEN` env var when set; otherwise a
/// fresh 32-byte random token is generated and persisted to `<db_dir>/.api_token`
/// with 0600 permissions. The token is printed to stdout on first generation so the
/// operator can copy it into the frontend's `VITE_API_TOKEN`.
#[derive(Clone)]
pub struct ApiToken {
    token: Arc<String>,
}

impl ApiToken {
    pub fn load_or_generate(db_path: &Path, env_token: Option<&str>) -> AppResult<Self> {
        if let Some(value) = env_token.map(str::trim).filter(|value| !value.is_empty()) {
            info!("API token loaded from AUTO_STONKS_API_TOKEN env var");
            return Ok(Self {
                token: Arc::new(value.to_string()),
            });
        }

        let token_path = token_file_path(db_path);
        if let Some(parent) = token_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|err| AppError::Internal(format!("create token dir: {err}")))?;
        }

        if let Ok(existing) = fs::read_to_string(&token_path) {
            let value = existing.trim().to_string();
            if !value.is_empty() {
                info!("API token loaded from {}", token_path.display());
                return Ok(Self {
                    token: Arc::new(value),
                });
            }
        }

        let mut bytes = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut bytes);
        let token = hex::encode(bytes);

        fs::write(&token_path, format!("{token}\n"))
            .map_err(|err| AppError::Internal(format!("write token file: {err}")))?;
        // Restrict to owner read/write only (Unix).
        #[cfg(unix)]
        {
            let permissions = fs::Permissions::from_mode(0o600);
            if let Err(err) = fs::set_permissions(&token_path, permissions) {
                warn!("could not tighten permissions on {}: {err}", token_path.display());
            }
        }
        #[cfg(not(unix))]
        {
            // On Windows the file inherits the ACL from its parent directory;
            // there is no simple chmod equivalent, so we skip the permission
            // restriction and rely on proper directory ACLs instead.
        }

        info!(
            "Generated new API token and wrote it to {}",
            token_path.display()
        );
        println!();
        println!("================ AutoStonks API token ================");
        println!("  {token}");
        println!("  Copy into frontend/.env as VITE_API_TOKEN=<token>");
        println!("  Or set AUTO_STONKS_API_TOKEN to pin an explicit value.");
        println!("======================================================");
        println!();

        Ok(Self {
            token: Arc::new(token),
        })
    }

    pub fn matches(&self, candidate: &str) -> bool {
        let expected = self.token.as_bytes();
        let provided = candidate.as_bytes();
        // Constant-time comparison avoids timing side-channels on token lookup.
        expected.ct_eq(provided).into()
    }
}

fn token_file_path(db_path: &Path) -> PathBuf {
    db_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".api_token")
}

#[derive(Debug, Deserialize)]
pub struct TokenQuery {
    token: Option<String>,
}

/// Axum middleware enforcing the bearer token on every request except `/api/health`.
///
/// Accepts either `Authorization: Bearer <token>` or a `?token=<token>` query
/// parameter. The query form exists because browsers cannot attach custom headers
/// to `EventSource` connections used by `/api/stream`.
pub async fn require_token(
    State(token): State<ApiToken>,
    Query(query): Query<TokenQuery>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let path = request.uri().path();
    if path == "/api/health" {
        return Ok(next.run(request).await);
    }

    let header_token = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::to_string);

    let candidate = header_token.or(query.token);
    let Some(candidate) = candidate else {
        return Err(StatusCode::UNAUTHORIZED);
    };

    if !token.matches(candidate.trim()) {
        return Err(StatusCode::UNAUTHORIZED);
    }

    Ok(next.run(request).await)
}
