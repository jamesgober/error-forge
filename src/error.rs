use std::fmt;
use std::path::PathBuf;
use std::io;

#[cfg(feature = "serde")]
use serde::Serialize;

/// Type alias for error-forge results.
pub type Result<T> = std::result::Result<T, crate::error::AppError>;

/// Base trait for all custom error variants.
pub trait ForgeError: std::error::Error + Send + Sync {
    fn kind(&self) -> &'static str;
    fn caption(&self) -> &'static str;
    fn is_retryable(&self) -> bool {
        false
    }
    fn is_fatal(&self) -> bool {
        true
    }
    fn status_code(&self) -> u16 {
        500
    }
    fn exit_code(&self) -> i32 {
        1
    }
}

/// Example enum until macro generates real one.
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum AppError {
    Config { message: String },
    Filesystem { path: Option<PathBuf>, source: io::Error },
    Other { message: String },
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config { message } => write!(f, "⚙️ Config: {}", message),
            Self::Filesystem { path, source } => write!(f, "💾 IO: {:?} ({})", path, source),
            Self::Other { message } => write!(f, "🚨 Error: {}", message),
        }
    }
}

impl std::error::Error for AppError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AppError::Filesystem { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl From<io::Error> for AppError {
    fn from(e: io::Error) -> Self {
        AppError::Filesystem { path: None, source: e }
    }
}