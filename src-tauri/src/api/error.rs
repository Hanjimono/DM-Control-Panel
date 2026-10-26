use std::collections::HashMap;

use super::{ApiError, ApiErrorKind};

/// Internal application error; mapped to [`ApiError`] for IPC responses.
#[derive(Debug)]
#[allow(dead_code)]
pub enum AppError {
    Validation {
        message: String,
        fields: Option<HashMap<String, String>>,
    },
    Expected(String),
    Unexpected(String),
}

#[allow(dead_code)]
impl AppError {
    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation {
            message: message.into(),
            fields: None,
        }
    }

    pub fn validation_with_fields(
        message: impl Into<String>,
        fields: HashMap<String, String>,
    ) -> Self {
        Self::Validation {
            message: message.into(),
            fields: Some(fields),
        }
    }

    pub fn expected(message: impl Into<String>) -> Self {
        Self::Expected(message.into())
    }

    pub fn unexpected(message: impl Into<String>) -> Self {
        Self::Unexpected(message.into())
    }
}

impl From<AppError> for ApiError {
    fn from(value: AppError) -> Self {
        match value {
            AppError::Validation { message, fields } => ApiError {
                kind: ApiErrorKind::Validation,
                message,
                fields,
            },
            AppError::Expected(message) => ApiError {
                kind: ApiErrorKind::Expected,
                message,
                fields: None,
            },
            AppError::Unexpected(message) => ApiError {
                kind: ApiErrorKind::Unexpected,
                message,
                fields: None,
            },
        }
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::Validation { message, .. } => write!(f, "validation: {message}"),
            AppError::Expected(message) => write!(f, "expected: {message}"),
            AppError::Unexpected(message) => write!(f, "unexpected: {message}"),
        }
    }
}

impl std::error::Error for AppError {}

impl From<rusqlite::Error> for AppError {
    fn from(value: rusqlite::Error) -> Self {
        AppError::unexpected(value.to_string())
    }
}
