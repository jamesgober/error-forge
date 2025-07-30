use std::fmt;
use std::path::PathBuf;
use std::io;
use std::error::Error as StdError;
use std::backtrace::Backtrace;

#[cfg(feature = "serde")]
use serde::Serialize;

/// Type alias for error-forge results.
pub type Result<T> = std::result::Result<T, crate::error::AppError>;

/// Base trait for all custom error variants.
pub trait ForgeError: std::error::Error + Send + Sync + 'static {
    /// Returns the kind of error, typically matching the enum variant
    fn kind(&self) -> &'static str;
    
    /// Returns a human-readable caption for the error
    fn caption(&self) -> &'static str;
    
    /// Returns true if the operation can be retried
    fn is_retryable(&self) -> bool {
        false
    }
    
    /// Returns true if the error is fatal and should terminate the program
    fn is_fatal(&self) -> bool {
        true
    }
    
    /// Returns an appropriate HTTP status code for the error
    fn status_code(&self) -> u16 {
        500
    }
    
    /// Returns an appropriate process exit code for the error
    fn exit_code(&self) -> i32 {
        1
    }
    
    /// Returns a user-facing message that can be shown to end users
    fn user_message(&self) -> String {
        self.to_string()
    }
    
    /// Returns a detailed technical message for developers/logs
    fn dev_message(&self) -> String {
        format!("[{}] {}", self.kind(), self)
    }
    
    /// Returns a backtrace if available
    fn backtrace(&self) -> Option<&Backtrace> {
        None
    }
    
    /// Registers the error with the central error registry
    fn register(&self) {
        crate::macros::call_error_hook(self.caption(), self.kind(), self.is_fatal(), self.is_retryable());
    }
}

/// Example error enum that can be replaced by the define_errors! macro.
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum AppError {
    /// Configuration-related errors
    Config { message: String },
    
    /// Filesystem-related errors with optional path and source error
    Filesystem { path: Option<PathBuf>, #[cfg_attr(feature = "serde", serde(skip))] source: io::Error },
    
    /// Network-related errors
    Network { endpoint: String, #[cfg_attr(feature = "serde", serde(skip))] source: Option<Box<dyn StdError + Send + Sync>> },
    
    /// Generic errors for anything not covered by specific variants
    Other { message: String },
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config { message } => write!(f, "⚙️ Configuration Error: {}", message),
            Self::Filesystem { path, source } => {
                if let Some(p) = path {
                    write!(f, "💾 Filesystem Error at {:?}: {}", p, source)
                } else {
                    write!(f, "💾 Filesystem Error: {}", source)
                }
            },
            Self::Network { endpoint, source } => {
                if let Some(src) = source {
                    write!(f, "🌐 Network Error on {}: {}", endpoint, src)
                } else {
                    write!(f, "🌐 Network Error on {}", endpoint)
                }
            },
            Self::Other { message } => write!(f, "🚨 Error: {}", message),
        }
    }
}

impl std::error::Error for AppError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AppError::Filesystem { source, .. } => Some(source),
            AppError::Network { source: Some(src), .. } => Some(src.as_ref()),
            _ => None,
        }
    }
}

impl From<io::Error> for AppError {
    fn from(e: io::Error) -> Self {
        AppError::Filesystem { path: None, source: e }
    }
}

impl ForgeError for AppError {
    fn kind(&self) -> &'static str {
        match self {
            Self::Config { .. } => "Config",
            Self::Filesystem { .. } => "Filesystem",
            Self::Network { .. } => "Network",
            Self::Other { .. } => "Other",
        }
    }
    
    fn caption(&self) -> &'static str {
        match self {
            Self::Config { .. } => "⚙️ Configuration",
            Self::Filesystem { .. } => "💾 Filesystem",
            Self::Network { .. } => "🌐 Network",
            Self::Other { .. } => "🚨 Error",
        }
    }
    
    fn is_retryable(&self) -> bool {
        matches!(self, Self::Network { .. })
    }
    
    fn status_code(&self) -> u16 {
        match self {
            Self::Config { .. } => 500,
            Self::Filesystem { .. } => 500,
            Self::Network { .. } => 503,
            Self::Other { .. } => 500,
        }
    }
}

/// Constructor methods for AppError
impl AppError {
    /// Create a new Config error
    pub fn config(message: impl Into<String>) -> Self {
        let instance = Self::Config { message: message.into() };
        crate::macros::call_error_hook(instance.caption(), instance.kind(), instance.is_fatal(), instance.is_retryable());
        instance
    }
    
    /// Create a new Filesystem error
    pub fn filesystem(path: impl Into<PathBuf>, source: io::Error) -> Self {
        let instance = Self::Filesystem { path: Some(path.into()), source };
        crate::macros::call_error_hook(instance.caption(), instance.kind(), instance.is_fatal(), instance.is_retryable());
        instance
    }
    
    /// Create a new Network error
    pub fn network(endpoint: impl Into<String>, source: Option<Box<dyn StdError + Send + Sync>>) -> Self {
        let instance = Self::Network { endpoint: endpoint.into(), source };
        crate::macros::call_error_hook(instance.caption(), instance.kind(), instance.is_fatal(), instance.is_retryable());
        instance
    }
    
    /// Create a new generic error
    pub fn other(message: impl Into<String>) -> Self {
        let instance = Self::Other { message: message.into() };
        crate::macros::call_error_hook(instance.caption(), instance.kind(), instance.is_fatal(), instance.is_retryable());
        instance
    }
}