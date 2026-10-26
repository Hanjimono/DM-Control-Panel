use super::{ApiResponse, AppError};
use serde::Serialize;

/// Runs a sync command body and always returns a typed [`ApiResponse`].
pub fn run_command<T, F>(name: &str, f: F) -> ApiResponse<T>
where
    T: Serialize,
    F: FnOnce() -> Result<T, AppError>,
{
    tracing::info!(cmd = name, "cmd start");
    match f() {
        Ok(data) => {
            tracing::info!(cmd = name, "cmd ok");
            ApiResponse::ok(data)
        }
        Err(err) => {
            tracing::error!(cmd = name, error = %err, "cmd err");
            ApiResponse::err(err.into())
        }
    }
}

/// Runs an async command body and always returns a typed [`ApiResponse`].
#[allow(dead_code)]
pub async fn run_command_async<T, F, Fut>(name: &str, f: F) -> ApiResponse<T>
where
    T: Serialize,
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<T, AppError>>,
{
    tracing::info!(cmd = name, "cmd start");
    match f().await {
        Ok(data) => {
            tracing::info!(cmd = name, "cmd ok");
            ApiResponse::ok(data)
        }
        Err(err) => {
            tracing::error!(cmd = name, error = %err, "cmd err");
            ApiResponse::err(err.into())
        }
    }
}
