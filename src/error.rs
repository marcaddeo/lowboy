use std::sync::Arc;

use axum::{http::StatusCode, response::IntoResponse};

#[derive(Debug, thiserror::Error)]
pub enum LowboyError {
    #[error("400 Bad Request")]
    BadRequest,

    #[error("401 Unauthorized")]
    Unauthorized,

    #[error("403 Forbidden")]
    Forbidden,

    #[error("404 Not Found")]
    NotFound,

    #[error("500 Internal Server Error")]
    Internal(#[from] anyhow::Error),
}

#[derive(Clone)]
pub(crate) struct ErrorWrapper(pub Arc<LowboyError>);

impl IntoResponse for LowboyError {
    fn into_response(self) -> axum::response::Response {
        use LowboyError::*;

        let code = match self {
            BadRequest => StatusCode::BAD_REQUEST,
            Unauthorized => StatusCode::UNAUTHORIZED,
            Forbidden => StatusCode::FORBIDDEN,
            NotFound => StatusCode::NOT_FOUND,
            Internal(ref inner) => {
                tracing::error!("Internal server error: {inner}");
                StatusCode::INTERNAL_SERVER_ERROR
            }
        };

        let mut response = (code, "").into_response();
        response
            .extensions_mut()
            .insert(ErrorWrapper(Arc::new(self)));

        response
    }
}
