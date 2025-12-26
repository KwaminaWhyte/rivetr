//! Unified API error handling for Rivetr.
//!
//! This module provides a consistent error response system across all API endpoints.
//! All errors are returned in a standard JSON format with appropriate HTTP status codes.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Error codes for API responses
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    // Client errors (4xx)
    BadRequest,
    Unauthorized,
    Forbidden,
    NotFound,
    MethodNotAllowed,
    Conflict,
    Gone,
    UnprocessableEntity,
    TooManyRequests,
    ValidationError,

    // Server errors (5xx)
    InternalError,
    ServiceUnavailable,
    DatabaseError,
    ExternalServiceError,
}

impl ErrorCode {
    /// Get the default HTTP status code for this error code
    pub fn status_code(&self) -> StatusCode {
        match self {
            ErrorCode::BadRequest => StatusCode::BAD_REQUEST,
            ErrorCode::Unauthorized => StatusCode::UNAUTHORIZED,
            ErrorCode::Forbidden => StatusCode::FORBIDDEN,
            ErrorCode::NotFound => StatusCode::NOT_FOUND,
            ErrorCode::MethodNotAllowed => StatusCode::METHOD_NOT_ALLOWED,
            ErrorCode::Conflict => StatusCode::CONFLICT,
            ErrorCode::Gone => StatusCode::GONE,
            ErrorCode::UnprocessableEntity => StatusCode::UNPROCESSABLE_ENTITY,
            ErrorCode::TooManyRequests => StatusCode::TOO_MANY_REQUESTS,
            ErrorCode::ValidationError => StatusCode::BAD_REQUEST,
            ErrorCode::InternalError => StatusCode::INTERNAL_SERVER_ERROR,
            ErrorCode::ServiceUnavailable => StatusCode::SERVICE_UNAVAILABLE,
            ErrorCode::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
            ErrorCode::ExternalServiceError => StatusCode::BAD_GATEWAY,
        }
    }

    /// Get the string representation of the error code
    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorCode::BadRequest => "bad_request",
            ErrorCode::Unauthorized => "unauthorized",
            ErrorCode::Forbidden => "forbidden",
            ErrorCode::NotFound => "not_found",
            ErrorCode::MethodNotAllowed => "method_not_allowed",
            ErrorCode::Conflict => "conflict",
            ErrorCode::Gone => "gone",
            ErrorCode::UnprocessableEntity => "unprocessable_entity",
            ErrorCode::TooManyRequests => "too_many_requests",
            ErrorCode::ValidationError => "validation_error",
            ErrorCode::InternalError => "internal_error",
            ErrorCode::ServiceUnavailable => "service_unavailable",
            ErrorCode::DatabaseError => "database_error",
            ErrorCode::ExternalServiceError => "external_service_error",
        }
    }
}

/// The inner error object in the response
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorBody {
    /// Machine-readable error code
    pub code: String,
    /// Human-readable error message
    pub message: String,
    /// Optional additional details (e.g., validation errors per field)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<ErrorDetails>,
}

/// Additional error details
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ErrorDetails {
    /// Field-level validation errors
    ValidationErrors(HashMap<String, Vec<String>>),
    /// Generic key-value details
    Generic(HashMap<String, serde_json::Value>),
}

/// The full error response envelope
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: ErrorBody,
}

/// Unified API error type
#[derive(Debug)]
pub struct ApiError {
    /// The error code
    code: ErrorCode,
    /// HTTP status code (defaults to code's default status)
    status: StatusCode,
    /// Human-readable error message
    message: String,
    /// Optional additional details
    details: Option<ErrorDetails>,
}

impl ApiError {
    /// Create a new API error with a specific code and message
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            status: code.status_code(),
            code,
            message: message.into(),
            details: None,
        }
    }

    /// Create an API error with a custom HTTP status code
    pub fn with_status(mut self, status: StatusCode) -> Self {
        self.status = status;
        self
    }

    /// Add details to the error
    pub fn with_details(mut self, details: ErrorDetails) -> Self {
        self.details = Some(details);
        self
    }

    /// Add validation errors as details
    pub fn with_validation_errors(mut self, errors: HashMap<String, Vec<String>>) -> Self {
        self.details = Some(ErrorDetails::ValidationErrors(errors));
        self
    }

    // -------------------------------------------------------------------------
    // Convenience constructors for common error types
    // -------------------------------------------------------------------------

    /// Bad request error (400)
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::BadRequest, message)
    }

    /// Unauthorized error (401) - authentication required
    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Unauthorized, message)
    }

    /// Forbidden error (403) - authenticated but not allowed
    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Forbidden, message)
    }

    /// Not found error (404)
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::NotFound, message)
    }

    /// Conflict error (409) - resource already exists or state conflict
    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Conflict, message)
    }

    /// Validation error (400) with field-level details
    pub fn validation(errors: HashMap<String, Vec<String>>) -> Self {
        let message = if errors.len() == 1 {
            errors.values().next().and_then(|v| v.first()).cloned()
                .unwrap_or_else(|| "Validation failed".to_string())
        } else {
            format!("Validation failed for {} fields", errors.len())
        };

        Self::new(ErrorCode::ValidationError, message)
            .with_validation_errors(errors)
    }

    /// Single field validation error
    pub fn validation_field(field: &str, message: impl Into<String>) -> Self {
        let mut errors = HashMap::new();
        errors.insert(field.to_string(), vec![message.into()]);
        Self::validation(errors)
    }

    /// Internal server error (500)
    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::InternalError, message)
    }

    /// Database error (500)
    pub fn database(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::DatabaseError, message)
    }

    /// Service unavailable error (503)
    pub fn service_unavailable(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::ServiceUnavailable, message)
    }

    /// Too many requests error (429)
    pub fn rate_limited(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::TooManyRequests, message)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let response = ErrorResponse {
            error: ErrorBody {
                code: self.code.as_str().to_string(),
                message: self.message,
                details: self.details,
            },
        };

        (self.status, Json(response)).into_response()
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code.as_str(), self.message)
    }
}

impl std::error::Error for ApiError {}

// -------------------------------------------------------------------------
// Conversion implementations for common error types
// -------------------------------------------------------------------------

impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        tracing::error!("Database error: {}", err);

        // Check for specific SQLx errors
        match &err {
            sqlx::Error::RowNotFound => {
                ApiError::not_found("Resource not found")
            }
            sqlx::Error::Database(db_err) => {
                // Check for constraint violations
                let msg = db_err.message();
                if msg.contains("UNIQUE constraint failed") {
                    ApiError::conflict("A resource with this identifier already exists")
                } else if msg.contains("FOREIGN KEY constraint failed") {
                    ApiError::bad_request("Referenced resource does not exist")
                } else {
                    ApiError::database("A database error occurred")
                }
            }
            _ => ApiError::database("A database error occurred"),
        }
    }
}

// -------------------------------------------------------------------------
// Builder for validation errors (integrates with existing validation module)
// -------------------------------------------------------------------------

/// Builder for collecting multiple validation errors
#[derive(Debug, Default)]
pub struct ValidationErrorBuilder {
    errors: HashMap<String, Vec<String>>,
}

impl ValidationErrorBuilder {
    /// Create a new validation error builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a validation error for a field
    pub fn add(&mut self, field: impl Into<String>, message: impl Into<String>) -> &mut Self {
        self.errors
            .entry(field.into())
            .or_default()
            .push(message.into());
        self
    }

    /// Check if there are any errors
    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    /// Build the ApiError if there are any errors
    pub fn build(self) -> Option<ApiError> {
        if self.errors.is_empty() {
            None
        } else {
            Some(ApiError::validation(self.errors))
        }
    }

    /// Return Ok(()) if no errors, or Err(ApiError) if there are errors
    pub fn finish(self) -> Result<(), ApiError> {
        match self.build() {
            Some(err) => Err(err),
            None => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_status_codes() {
        assert_eq!(ErrorCode::BadRequest.status_code(), StatusCode::BAD_REQUEST);
        assert_eq!(ErrorCode::Unauthorized.status_code(), StatusCode::UNAUTHORIZED);
        assert_eq!(ErrorCode::NotFound.status_code(), StatusCode::NOT_FOUND);
        assert_eq!(ErrorCode::Conflict.status_code(), StatusCode::CONFLICT);
        assert_eq!(ErrorCode::InternalError.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(ErrorCode::TooManyRequests.status_code(), StatusCode::TOO_MANY_REQUESTS);
    }

    #[test]
    fn test_api_error_creation() {
        let err = ApiError::not_found("App not found");
        assert_eq!(err.code, ErrorCode::NotFound);
        assert_eq!(err.status, StatusCode::NOT_FOUND);
        assert_eq!(err.message, "App not found");
    }

    #[test]
    fn test_validation_error_single_field() {
        let err = ApiError::validation_field("name", "Name is required");
        assert_eq!(err.code, ErrorCode::ValidationError);
        assert!(err.message.contains("Name is required"));
    }

    #[test]
    fn test_validation_error_multiple_fields() {
        let mut errors = HashMap::new();
        errors.insert("name".to_string(), vec!["Name is required".to_string()]);
        errors.insert("email".to_string(), vec!["Invalid email format".to_string()]);

        let err = ApiError::validation(errors);
        assert_eq!(err.code, ErrorCode::ValidationError);
        assert!(err.message.contains("2 fields"));
    }

    #[test]
    fn test_validation_error_builder() {
        let mut builder = ValidationErrorBuilder::new();
        builder.add("name", "Name is required");
        builder.add("email", "Invalid email format");
        builder.add("name", "Name is too short");

        assert!(!builder.is_empty());

        let err = builder.build().unwrap();
        assert_eq!(err.code, ErrorCode::ValidationError);

        if let Some(ErrorDetails::ValidationErrors(errors)) = &err.details {
            assert_eq!(errors.get("name").unwrap().len(), 2);
            assert_eq!(errors.get("email").unwrap().len(), 1);
        } else {
            panic!("Expected ValidationErrors details");
        }
    }

    #[test]
    fn test_custom_status_code() {
        let err = ApiError::new(ErrorCode::InternalError, "Custom error")
            .with_status(StatusCode::BAD_GATEWAY);

        assert_eq!(err.status, StatusCode::BAD_GATEWAY);
        assert_eq!(err.code, ErrorCode::InternalError);
    }
}
