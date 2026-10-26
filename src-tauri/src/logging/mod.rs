use crate::config::LogLevel;
use std::path::{Path, PathBuf};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::fmt::time::UtcTime;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, EnvFilter};

/// Keeps the non-blocking appender worker alive for the process lifetime.
pub struct LogGuard(#[allow(dead_code)] WorkerGuard);

/// Resolves the log directory: project `src-tauri/logs` in debug, `{app_data_dir}/logs` in release.
pub fn resolve_logs_dir(app_data_dir: &Path) -> PathBuf {
    if cfg!(debug_assertions) {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("logs")
    } else {
        app_data_dir.join("logs")
    }
}

/// Initializes tracing: daily-rolling file under `logs_dir` plus stderr, filtered by `level`.
///
/// The returned [`LogGuard`] must be retained (e.g. via `app.manage`) or the file writer stops.
pub fn init(logs_dir: &Path, level: LogLevel) -> Result<LogGuard, LoggingError> {
    std::fs::create_dir_all(logs_dir).map_err(|source| LoggingError::CreateDir {
        path: logs_dir.display().to_string(),
        source,
    })?;

    let file_appender = tracing_appender::rolling::daily(logs_dir, "app.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let filter =
        EnvFilter::try_new(level.as_filter_directive()).map_err(|source| LoggingError::Filter {
            directive: level.as_filter_directive().to_string(),
            source,
        })?;

    let timer = UtcTime::rfc_3339();

    let file_layer = fmt::layer()
        .with_ansi(false)
        .with_target(true)
        .with_timer(timer.clone())
        .with_writer(non_blocking);

    let stderr_layer = fmt::layer()
        .with_ansi(false)
        .with_target(true)
        .with_timer(timer)
        .with_writer(std::io::stderr);

    tracing_subscriber::registry()
        .with(filter)
        .with(file_layer)
        .with(stderr_layer)
        .try_init()
        .map_err(|source| LoggingError::Init { source })?;

    Ok(LogGuard(guard))
}

#[derive(Debug)]
pub enum LoggingError {
    CreateDir {
        path: String,
        source: std::io::Error,
    },
    Filter {
        directive: String,
        source: tracing_subscriber::filter::ParseError,
    },
    Init {
        source: tracing_subscriber::util::TryInitError,
    },
}

impl std::fmt::Display for LoggingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoggingError::CreateDir { path, source } => {
                write!(f, "failed to create logs dir {path}: {source}")
            }
            LoggingError::Filter { directive, source } => {
                write!(f, "invalid log filter '{directive}': {source}")
            }
            LoggingError::Init { source } => {
                write!(f, "failed to initialize tracing subscriber: {source}")
            }
        }
    }
}

impl std::error::Error for LoggingError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            LoggingError::CreateDir { source, .. } => Some(source),
            LoggingError::Filter { source, .. } => Some(source),
            LoggingError::Init { source } => Some(source),
        }
    }
}
