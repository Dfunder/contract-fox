//! HTTP-layer error type for the public REST API.
//!
//! Every variant has an explicit [`axum::http::StatusCode`] mapping via the
//! [`IntoResponse`] impl. New variants must specify a status code here or the
//! compile will fail because [`ApiError::into_response`] is non-exhaustive.

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use thiserror::Error;

use crate::errors::StellarAidError;

/// Machine-readable error codes returned to clients.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiErrorCode {
    /// Client supplied invalid or missing fields.
    ValidationFailed,
    /// Server-side issue (RPC, DB, network).
    InternalError,
}

/// Application error that knows how to render itself as an HTTP response.
#[derive(Debug, Error)]
pub enum ApiError {
    /// 400 Bad Request – the request body was malformed or invalid.
    #[error("{message}")]
    BadRequest { message: String },

    /// 500 Internal Server Error – the server failed to carry out the request.
    #[error("{message}")]
    Internal { message: String },
}

impl ApiError {
    /// Build a 400-class error with a public-facing message.
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::BadRequest {
            message: message.into(),
        }
    }

    /// Build a 500-class error with a public-facing message.
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
        }
    }
}

#[derive(Serialize)]
struct ErrorBody {
    error: ApiErrorCode,
    message: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code) = match &self {
            ApiError::BadRequest { .. } => {
                (StatusCode::BAD_REQUEST, ApiErrorCode::ValidationFailed)
            }
            ApiError::Internal { .. } => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiErrorCode::InternalError,
            ),
        };

        let body = ErrorBody {
            error: code,
            message: self.to_string(),
        };
        (status, Json(body)).into_response()
    }
}

impl From<StellarAidError> for ApiError {
    fn from(err: StellarAidError) -> Self {
        match err {
            StellarAidError::ValidationError(_) => Self::bad_request(err.to_string()),
            // `TransactionFailed` represents a *network* failure of the user's
            // submission – the request itself was valid, so we map to 502.
            StellarAidError::TransactionFailed(_) => Self::internal(err.to_string()),
            _ => Self::internal(err.to_string()),
        }
    }
}

impl From<crate::soroban::rpc_client::RpcError> for ApiError {
    fn from(err: crate::soroban::rpc_client::RpcError) -> Self {
        // RPC-level failures are server-side issues, not client validation.
        Self::internal(format!("rpc: {err}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bad_request_message_included_in_display() {
        let err = ApiError::bad_request("missing field x");
        assert_eq!(err.to_string(), "missing field x");
    }

    #[test]
    fn internal_message_included_in_display() {
        let err = ApiError::internal("db down");
        assert_eq!(err.to_string(), "db down");
    }

    #[test]
    fn validation_error_maps_to_bad_request_variant() {
        let err: ApiError = StellarAidError::ValidationError("bad amount".into()).into();
        assert!(matches!(err, ApiError::BadRequest { .. }));
        assert!(err.to_string().contains("bad amount"));
    }

    #[test]
    fn database_error_maps_to_internal_variant() {
        let err: ApiError = StellarAidError::DatabaseError("connection lost".into()).into();
        assert!(matches!(err, ApiError::Internal { .. }));
        assert!(err.to_string().contains("connection lost"));
    }
}
