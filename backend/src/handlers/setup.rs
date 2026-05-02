use std::fs;
use std::path::PathBuf;

use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};
use subtle::ConstantTimeEq;

use crate::error::{ApiResponse, AppError, AppResult};
use crate::AppState;

#[derive(Debug, Serialize)]
pub struct SetupStatusResponse {
    pub backend_ready: bool,
    pub has_credentials: bool,
}

pub async fn setup_status(State(state): State<AppState>) -> ApiResponse<SetupStatusResponse> {
    let has_credentials = {
        let db = state.db.lock().await;
        db.count_credentials().unwrap_or(0) > 0
    };

    ApiResponse {
        success: true,
        data: Some(SetupStatusResponse {
            backend_ready: true,
            has_credentials,
        }),
        error: None,
    }
}

#[derive(Debug, Deserialize)]
pub struct WriteEnvRequest {
    pub api_token: String,
}

#[derive(Debug, Serialize)]
pub struct WriteEnvResponse {
    pub written: bool,
    pub message: String,
}

pub async fn write_env(
    State(state): State<AppState>,
    Json(payload): Json<WriteEnvRequest>,
) -> AppResult<ApiResponse<WriteEnvResponse>> {
    let candidate = payload.api_token.trim().to_string();
    if candidate.is_empty() {
        return Err(AppError::Validation(
            "api_token must not be empty".to_string(),
        ));
    }

    // Constant-time comparison with the actual API token stored in AppState
    let expected = state.api_token.as_bytes();
    let provided = candidate.as_bytes();
    let matches: bool = expected.ct_eq(provided).into();
    if !matches {
        return Err(AppError::Unauthorized("Invalid API token".to_string()));
    }

    let env_path = resolve_frontend_env_path(&state)?;

    let content = match fs::read_to_string(&env_path) {
        Ok(existing) => rewrite_env_token(existing, &candidate),
        Err(_) => {
            if let Some(parent) = env_path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|err| AppError::Internal(format!("create frontend dir: {err}")))?;
            }
            format!("VITE_API_BASE_URL=\nVITE_API_TOKEN={candidate}\n")
        }
    };

    fs::write(&env_path, content)
        .map_err(|err| AppError::Internal(format!("write frontend/.env: {err}")))?;

    tracing::info!("Wrote VITE_API_TOKEN to {}", env_path.display());

    Ok(ApiResponse {
        success: true,
        data: Some(WriteEnvResponse {
            written: true,
            message: format!("VITE_API_TOKEN saved to {}", env_path.display()),
        }),
        error: None,
    })
}

fn resolve_frontend_env_path(state: &AppState) -> AppResult<PathBuf> {
    let db_dir = state.config.database_path.parent();
    let project_root = db_dir
        .and_then(|p| p.parent())
        .unwrap_or_else(|| std::path::Path::new("."));

    let path = project_root.join("frontend").join(".env");

    if path.parent().is_some() {
        return Ok(path);
    }

    Err(AppError::Internal(
        "cannot resolve frontend/.env path".to_string(),
    ))
}

/// Replace or append the VITE_API_TOKEN line in existing .env content.
fn rewrite_env_token(existing: String, token: &str) -> String {
    let mut found = false;
    let lines: Vec<String> = existing
        .lines()
        .map(|line| {
            if line.trim_start().starts_with("VITE_API_TOKEN=") {
                found = true;
                format!("VITE_API_TOKEN={token}")
            } else {
                line.to_string()
            }
        })
        .collect();

    if found {
        lines.join("\n") + "\n"
    } else {
        let mut out = lines.join("\n");
        if !out.ends_with('\n') {
            out.push('\n');
        }
        out.push_str(&format!("VITE_API_TOKEN={token}\n"));
        out
    }
}
