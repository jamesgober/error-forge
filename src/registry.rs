use crate::error::ForgeError;
use std::collections::HashMap;
use std::fmt;
use std::sync::OnceLock;
use std::sync::RwLock;

/// A central registry for error codes and metadata
pub struct ErrorRegistry {
    /// Maps error codes to their descriptions
    codes: RwLock<HashMap<String, ErrorCodeInfo>>,
}

/// Metadata for a registered error code
#[derive(Clone, Debug)]
pub struct ErrorCodeInfo {
    /// The error code (e.g. "AUTH-001")
    pub code: String,
    /// A human-readable description of this error type
    pub description: String,
    /// A URL to documentation about this error, if available
    pub documentation_url: Option<String>,
    /// Whether this error is expected to be retryable
    pub retryable: bool,
}

impl ErrorRegistry {
    /// Create a new empty error registry
    fn new() -> Self {
        Self {
            codes: RwLock::new(HashMap::new()),
        }
    }

    /// Register an error code with metadata
    pub fn register_code(
        &self,
        code: String,
        description: String,
        documentation_url: Option<String>,
        retryable: bool,
    ) -> Result<(), String> {
        let mut codes = match self.codes.write() {
            Ok(codes) => codes,
            Err(_) => return Err("Failed to acquire write lock on error registry".to_string()),
        };

        if codes.contains_key(&code) {
            return Err(format!("Error code '{code}' is already registered"));
        }

        codes.insert(
            code.clone(),
            ErrorCodeInfo {
                code,
                description,
                documentation_url,
                retryable,
            },
        );

        Ok(())
    }

    /// Get info about a registered error code
    pub fn get_code_info(&self, code: &str) -> Option<ErrorCodeInfo> {
        match self.codes.read() {
            Ok(codes) => codes.get(code).cloned(),
            Err(_) => None,
        }
    }

    /// Check if an error code is registered
    pub fn is_registered(&self, code: &str) -> bool {
        match self.codes.read() {
            Ok(codes) => codes.contains_key(code),
            Err(_) => false,
        }
    }

    /// Get the global error registry instance
    pub fn global() -> &'static ErrorRegistry {
        static REGISTRY: OnceLock<ErrorRegistry> = OnceLock::new();
        REGISTRY.get_or_init(ErrorRegistry::new)
    }
}

/// An error with an associated error code
#[derive(Debug)]
pub struct CodedError<E> {
    /// The original error
    pub error: E,
    /// The error code
    pub code: String,
    /// Per-instance override for retryability
    pub retryable: Option<bool>,
    /// Whether this error is fatal
    pub fatal: bool,
    /// Per-instance override for status code
    pub status: Option<u16>,
}

impl<E> CodedError<E> {
    /// Create a new coded error
    pub fn new(error: E, code: impl Into<String>) -> Self {
        let code = code.into();

        // Register the code if it's not already registered
        if !ErrorRegistry::global().is_registered(&code) {
            let _ = register_error_code(
                code.clone(),
                format!("Error code {code}"),
                None as Option<String>,
                false,
            );
        }

        Self {
            error,
            code,
            retryable: None,
            fatal: false,
            status: None,
        }
    }

    /// Get information about this error code from the registry
    pub fn code_info(&self) -> Option<ErrorCodeInfo> {
        ErrorRegistry::global().get_code_info(&self.code)
    }

    /// Set whether this error is retryable
    pub fn with_retryable(mut self, retryable: bool) -> Self {
        self.retryable = Some(retryable);
        self
    }

    /// Set whether this error is fatal
    pub fn with_fatal(mut self, fatal: bool) -> Self {
        self.fatal = fatal;
        self
    }

    /// Set the HTTP status code for this error
    pub fn with_status(mut self, status: u16) -> Self {
        self.status = Some(status);
        self
    }
}

impl<E: fmt::Display> fmt::Display for CodedError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.code, self.error)
    }
}

impl<E: std::error::Error + 'static> std::error::Error for CodedError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}

// Implement ForgeError for CodedError when the inner error implements ForgeError
impl<E: ForgeError> ForgeError for CodedError<E> {
    fn kind(&self) -> &'static str {
        self.error.kind()
    }

    fn caption(&self) -> &'static str {
        self.error.caption()
    }

    fn is_retryable(&self) -> bool {
        self.retryable.unwrap_or_else(|| {
            self.code_info()
                .map_or_else(|| self.error.is_retryable(), |info| info.retryable)
        })
    }

    fn is_fatal(&self) -> bool {
        self.fatal || self.error.is_fatal()
    }

    fn status_code(&self) -> u16 {
        self.status.unwrap_or_else(|| self.error.status_code())
    }

    fn exit_code(&self) -> i32 {
        self.error.exit_code()
    }

    fn user_message(&self) -> String {
        format!("[{}] {}", self.code, self.error.user_message())
    }

    fn dev_message(&self) -> String {
        if let Some(info) = self.code_info() {
            if let Some(url) = info.documentation_url {
                return format!("[{}] {} ({})", self.code, self.error.dev_message(), url);
            }
        }
        format!("[{}] {}", self.code, self.error.dev_message())
    }

    fn backtrace(&self) -> Option<&std::backtrace::Backtrace> {
        self.error.backtrace()
    }
}

/// Extension trait for adding error codes
pub trait WithErrorCode<E> {
    /// Attach an error code to an error
    fn with_code(self, code: impl Into<String>) -> CodedError<E>;
}

impl<E> WithErrorCode<E> for E {
    fn with_code(self, code: impl Into<String>) -> CodedError<E> {
        CodedError::new(self, code)
    }
}

/// Register an error code in the global registry
pub fn register_error_code(
    code: impl Into<String>,
    description: impl Into<String>,
    documentation_url: Option<impl Into<String>>,
    retryable: bool,
) -> Result<(), String> {
    ErrorRegistry::global().register_code(
        code.into(),
        description.into(),
        documentation_url.map(|url| url.into()),
        retryable,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AppError;

    #[test]
    fn test_error_with_code() {
        let error = AppError::config("Invalid config").with_code("CONFIG-001");

        assert_eq!(
            error.to_string(),
            "[CONFIG-001] ⚙️ Configuration Error: Invalid config"
        );
    }

    #[test]
    fn test_register_error_code() {
        let _ = register_error_code(
            "AUTH-001",
            "Authentication failed due to invalid credentials",
            Some("https://docs.example.com/errors/auth-001"),
            true,
        );

        let info = ErrorRegistry::global().get_code_info("AUTH-001");
        assert!(info.is_some());
        let info = info.unwrap();
        assert_eq!(info.code, "AUTH-001");
        assert_eq!(
            info.description,
            "Authentication failed due to invalid credentials"
        );
        assert_eq!(
            info.documentation_url,
            Some("https://docs.example.com/errors/auth-001".to_string())
        );
        assert!(info.retryable);
    }
}
