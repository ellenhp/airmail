use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use log::warn;
use thiserror::Error;

#[derive(Error, Debug)]
#[allow(clippy::module_name_repetitions)]
pub enum AirmailServiceError {
    #[error("general error: `{0}`")]
    InternalAnyhowError(Box<anyhow::Error>),

    #[error("failed to encode response")]
    SerdeEncodeError(#[from] serde_json::Error),
}

// Tell axum how to convert `AppError` into a response.
impl IntoResponse for AirmailServiceError {
    fn into_response(self) -> Response {
        match &self {
            Self::InternalAnyhowError(e) => {
                warn!("InternalAnyhowError: {:#}", self);
                (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
            }
            Self::SerdeEncodeError(e) => {
                warn!("SerdeEncodeError: {:#}", self);
                (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
            } // _ => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()).into_response(),
        }
    }
}

impl From<anyhow::Error> for AirmailServiceError {
    fn from(e: anyhow::Error) -> Self {
        Self::InternalAnyhowError(Box::new(e))
    }
}
