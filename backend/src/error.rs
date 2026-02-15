use axum::{http::StatusCode, response::IntoResponse, Json};
use serde::Serialize;
use tracing::{error, warn};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("unauthorized")]
    Unauthorized,
    #[error("forbidden")]
    Forbidden,
    #[error("too many requests: {0}")]
    TooManyRequests(String),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("internal server error")]
    Internal(anyhow::Error),
}

impl AppError {
    pub fn internal<E>(err: E) -> Self
    where
        E: Into<anyhow::Error>,
    {
        Self::Internal(err.into())
    }

    fn code(&self) -> &'static str {
        match self {
            Self::BadRequest(_) => "bad_request",
            Self::Unauthorized => "unauthorized",
            Self::Forbidden => "forbidden",
            Self::TooManyRequests(_) => "too_many_requests",
            Self::Conflict(_) => "conflict",
            Self::Internal(_) => "internal_error",
        }
    }

    fn status(&self) -> StatusCode {
        match self {
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::TooManyRequests(_) => StatusCode::TOO_MANY_REQUESTS,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn message(&self) -> String {
        match self {
            Self::BadRequest(msg) => msg.clone(),
            Self::Unauthorized => "invalid credentials or token".to_string(),
            Self::Forbidden => "permission denied".to_string(),
            Self::TooManyRequests(msg) => msg.clone(),
            Self::Conflict(msg) => msg.clone(),
            Self::Internal(_) => "unexpected server error".to_string(),
        }
    }
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    code: &'static str,
    message: String,
}

#[derive(Debug, Serialize)]
struct ErrorEnvelope {
    error: ErrorBody,
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let status = self.status();
        let code = self.code();
        let message = self.message();

        match &self {
            Self::Internal(err) => {
                error!(
                    status = status.as_u16(),
                    code,
                    message = %message,
                    error = ?err,
                    "api request failed with internal error"
                );
            }
            _ => {
                warn!(
                    status = status.as_u16(),
                    code,
                    message = %message,
                    "api request rejected"
                );
            }
        }

        let body = ErrorEnvelope {
            error: ErrorBody { code, message },
        };

        (status, Json(body)).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;
