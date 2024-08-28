use axum::body::Body;
use axum::response::IntoResponse;
use axum::response::Response;
use http::header::ToStrError;
use http::StatusCode;
use thiserror::Error;

use super::IntoErrorResponse;
#[derive(Debug, Error)]
pub enum BadRequestErrors {
    #[error("Could not Decode Base64: {0}")]
    Base64DecodeError(#[from] base64::DecodeError),
    #[error("Invalid UTF-8: {0}")]
    InvalidUTF8(#[from] std::string::FromUtf8Error),
    #[error("Invalid Header: {0}")]
    InvalidHeader(#[from] ToStrError),
    #[error("Invalid Header Time: {0}")]
    InvalidHeaderTime(#[from] chrono::ParseError),
    #[error(transparent)]
    InvalidAuthorizationHeader(#[from] InvalidAuthorizationHeader),
    #[error("{0}")]
    Other(String),
    #[error(transparent)]
    Axum(#[from] axum::Error),
    #[error("Missing Header: {0}")]
    MissingHeader(&'static str),
    #[error("Invalid Json Request: {0}")]
    InvalidJson(#[from] serde_json::Error),
}
impl IntoErrorResponse for BadRequestErrors {
    fn into_response_boxed(self: Box<Self>) -> axum::response::Response {
        self.into_response()
    }
}
#[derive(Debug, Error)]
pub enum InvalidAuthorizationHeader {
    #[error("Invalid Authorization Scheme")]
    InvalidScheme,
    #[error("Invalid Authorization Value")]
    InvalidValue,
    #[error("Invalid Authorization Format. Expected: (Schema Type) (Value)")]
    InvalidFormat,
    #[error("Invalid Basic Authorization Value Expected: base64(username:password)")]
    InvalidBasicValue,
}

impl IntoResponse for BadRequestErrors {
    fn into_response(self) -> axum::response::Response {
        Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(self.to_string()))
            .unwrap()
    }
}
