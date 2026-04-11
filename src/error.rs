use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("API Error: {0}")]
    ApiError(#[from] reqwest::Error),

    #[error("Authentication Error: {0}")]
    AuthError(String),

    #[error("Validation Error: {0}")]
    ValidationError(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Internal Server Error: {0}")]
    Internal(String),

    #[error("Not Found: {0}")]
    NotFound(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::ApiError(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            AppError::AuthError(e) => (StatusCode::UNAUTHORIZED, e),
            AppError::ValidationError(e) => (StatusCode::BAD_REQUEST, e),
            AppError::Unauthorized(e) => (StatusCode::UNAUTHORIZED, e),
            AppError::Forbidden(e) => (StatusCode::FORBIDDEN, e),
            AppError::Internal(e) => (StatusCode::INTERNAL_SERVER_ERROR, e),
            AppError::NotFound(e) => (StatusCode::NOT_FOUND, e),
        };

        let body = Json(json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;
    use axum::response::IntoResponse;

    #[test]
    fn test_app_error_into_response() {
        let err = AppError::ValidationError("test validation error".to_string());
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let err = AppError::Unauthorized("test unauthorized".to_string());
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        let err = AppError::Forbidden("test forbidden".to_string());
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        let err = AppError::NotFound("test not found".to_string());
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
