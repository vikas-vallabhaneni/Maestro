use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

pub enum AppError {
    Internal(anyhow::Error),
    NotFound(String),
}

impl AppError {
    #[must_use]
    pub fn not_found(msg: &str) -> Self {
        Self::NotFound(msg.to_string())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            Self::Internal(err) => {
                tracing::error!(error = %err, "internal server error");
                (StatusCode::INTERNAL_SERVER_ERROR, "internal server error").into_response()
            }
            Self::NotFound(msg) => (StatusCode::NOT_FOUND, msg).into_response(),
        }
    }
}

impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self::Internal(err.into())
    }
}
