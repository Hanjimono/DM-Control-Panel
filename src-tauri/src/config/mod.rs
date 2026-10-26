use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Default config shipped with the app (`src-tauri/config.toml`).
pub const DEFAULT_CONFIG: &str = include_str!("../../config.toml");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: LogLevel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Info,
    Debug,
    Trace,
}

impl LogLevel {
    /// Filter directive for `tracing_subscriber::EnvFilter`.
    pub fn as_filter_directive(self) -> &'static str {
        match self {
            LogLevel::Info => "info",
            LogLevel::Debug => "debug",
            LogLevel::Trace => "trace",
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            logging: LoggingConfig {
                level: LogLevel::Info,
            },
        }
    }
}

/// Loads config from `path`, or seeds it and returns defaults if missing.
///
/// When `bundled_default` exists (production resource / packaged `config.toml`),
/// it is copied into `path`. Otherwise the embedded [`DEFAULT_CONFIG`] is written.
pub fn load_or_create(
    path: &Path,
    bundled_default: Option<&Path>,
) -> Result<AppConfig, ConfigError> {
    if path.exists() {
        return read_config(path);
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| ConfigError::Write {
            path: path.display().to_string(),
            source,
        })?;
    }

    match bundled_default {
        Some(src) if src.exists() => {
            fs::copy(src, path).map_err(|source| ConfigError::Write {
                path: path.display().to_string(),
                source,
            })?;
        }
        _ => {
            fs::write(path, DEFAULT_CONFIG).map_err(|source| ConfigError::Write {
                path: path.display().to_string(),
                source,
            })?;
        }
    }

    read_config(path)
}

fn read_config(path: &Path) -> Result<AppConfig, ConfigError> {
    let contents = fs::read_to_string(path).map_err(|source| ConfigError::Read {
        path: path.display().to_string(),
        source,
    })?;
    toml::from_str(&contents).map_err(|source| ConfigError::Parse {
        path: path.display().to_string(),
        source,
    })
}

#[derive(Debug)]
pub enum ConfigError {
    Read {
        path: String,
        source: std::io::Error,
    },
    Write {
        path: String,
        source: std::io::Error,
    },
    Parse {
        path: String,
        source: toml::de::Error,
    },
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::Read { path, source } => {
                write!(f, "failed to read config at {path}: {source}")
            }
            ConfigError::Write { path, source } => {
                write!(f, "failed to write config at {path}: {source}")
            }
            ConfigError::Parse { path, source } => {
                write!(f, "failed to parse config at {path}: {source}")
            }
        }
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ConfigError::Read { source, .. } | ConfigError::Write { source, .. } => Some(source),
            ConfigError::Parse { source, .. } => Some(source),
        }
    }
}
