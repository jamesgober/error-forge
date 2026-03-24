use error_forge::{define_errors, AppError, ForgeError};
use std::error::Error;
use std::io;

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
struct TestSourceError {
    message: &'static str,
}

impl std::fmt::Display for TestSourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for TestSourceError {}

impl error_forge::macros::ErrorSource for TestSourceError {
    fn as_source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self)
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
struct OptionalTestSource(Option<TestSourceError>);

impl std::fmt::Display for OptionalTestSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            Some(source) => write!(f, "{}", source),
            None => write!(f, "no source"),
        }
    }
}

impl error_forge::macros::ErrorSource for OptionalTestSource {
    fn as_source(&self) -> Option<&(dyn Error + 'static)> {
        self.0.as_ref().map(|error| error as &(dyn Error + 'static))
    }
}

define_errors! {
    pub enum MacroGeneratedError {
        #[error(display = "Filesystem access failed for {path}", path)]
        #[kind(Filesystem, retryable = true, status = 503)]
        Filesystem { path: String, source: TestSourceError },

        #[error(display = "Network request failed for {endpoint}", endpoint)]
        #[kind(Network, retryable = true, status = 502)]
        Network { endpoint: String, source: OptionalTestSource },

        #[kind(Config)]
        MissingConfig,
    }
}

#[test]
fn test_error_display() {
    // Create a config error
    let config_err = AppError::config("Missing configuration file");

    // Test basic ForgeError trait methods
    assert_eq!(config_err.kind(), "Config");
    assert_eq!(config_err.caption(), "⚙️ Configuration");
    assert!(!config_err.is_retryable());

    // Verify the error display formatting
    let display_str = format!("{}", config_err);
    assert!(display_str.contains("⚙️ Configuration Error"));
    assert!(display_str.contains("Missing configuration file"));
}

#[test]
fn test_filesystem_error() {
    let io_err = io::Error::new(io::ErrorKind::NotFound, "File not found");
    let fs_err = AppError::filesystem("config.json", io_err);

    // Test error metadata
    assert_eq!(fs_err.kind(), "Filesystem");
    assert!(fs_err.status_code() >= 500);

    // Verify error chaining via source()
    let source = fs_err.source();
    assert!(source.is_some());

    // Check display formatting with emoji
    let display_str = format!("{}", fs_err);
    assert!(display_str.contains("💾 Filesystem Error"));
    assert!(display_str.contains("config.json"));
}

#[test]
fn test_network_error() {
    // Test error with optional source
    let net_err = AppError::network("https://api.example.com", None);
    assert_eq!(net_err.kind(), "Network");

    // Check display formatting
    let display_str = format!("{}", net_err);
    assert!(display_str.contains("🌐 Network Error"));
    assert!(display_str.contains("https://api.example.com"));

    // Add a source error
    let source_err = io::Error::new(io::ErrorKind::ConnectionRefused, "Connection failed");

    // Using a simpler approach - construct with None and check source() exists
    let net_err2 = AppError::from(source_err);

    // An alternative approach would be to use the network constructor directly with proper casting:
    // let net_err2 = AppError::network(
    //     "https://api.example.com",
    //     Some(Box::new(source_err) as Box<dyn Error + Send + Sync>)
    // );

    // Verify source error is properly chained
    assert!(net_err2.source().is_some());
}

#[test]
fn test_define_errors_source_chaining() {
    let error = MacroGeneratedError::filesystem(
        "config.json".to_string(),
        TestSourceError {
            message: "Missing config",
        },
    );

    assert_eq!(error.kind(), "Filesystem");
    assert!(error.is_retryable());
    assert_eq!(error.status_code(), 503);
    assert!(!error.is_fatal());
    assert!(error
        .to_string()
        .contains("Filesystem access failed for config.json"));
    assert_eq!(error.source().unwrap().to_string(), "Missing config");
}

#[test]
fn test_define_errors_optional_source_chaining() {
    let error = MacroGeneratedError::network(
        "https://api.example.com".to_string(),
        OptionalTestSource(Some(TestSourceError {
            message: "Connection reset",
        })),
    );

    assert_eq!(error.kind(), "Network");
    assert!(error.is_retryable());
    assert_eq!(error.status_code(), 502);
    assert!(error.to_string().contains("https://api.example.com"));
    assert_eq!(error.source().unwrap().to_string(), "Connection reset");
}

#[test]
fn test_define_errors_default_fatal_is_false() {
    let error = MacroGeneratedError::missingconfig();

    assert_eq!(error.kind(), "Config");
    assert!(!error.is_fatal());
}
