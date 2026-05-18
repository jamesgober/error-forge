//! Console theming for error display in CLI applications.
//!
//! This module provides ANSI color formatting for error messages
//! displayed in terminal environments. It auto-detects terminal
//! capabilities via [`std::io::IsTerminal`] and disables colors when
//! stderr is not a TTY, when `TERM=dumb`, or when `NO_COLOR` is set
//! (<https://no-color.org/>).

use std::io::IsTerminal;

/// Color theme for console error output.
///
/// The fields are `&'static str` ANSI escapes — no allocation per
/// construction, and `const`-constructible for the three preset
/// constructors ([`ConsoleTheme::with_colors`], [`ConsoleTheme::plain`]).
pub struct ConsoleTheme {
    error_color: &'static str,
    warning_color: &'static str,
    info_color: &'static str,
    success_color: &'static str,
    caption_color: &'static str,
    reset: &'static str,
    bold: &'static str,
    dim: &'static str,
}

/// Detect if the current terminal supports ANSI colors.
fn terminal_supports_ansi() -> bool {
    // Cache the answer for the process. The decision is based on
    // env vars + the `stderr` handle, both of which are effectively
    // process-static, so caching is correct (and removes per-call
    // env-var reads from the hot path).
    static SUPPORTS_ANSI: std::sync::OnceLock<bool> = std::sync::OnceLock::new();

    *SUPPORTS_ANSI.get_or_init(|| {
        // Stderr must be a terminal — applies to every platform.
        if !std::io::stderr().is_terminal() {
            return false;
        }

        // Explicit terminal-dumb signal.
        if let Ok(term) = std::env::var("TERM") {
            if term == "dumb" {
                return false;
            }
        }

        // <https://no-color.org/>: any non-empty `NO_COLOR` disables.
        if std::env::var_os("NO_COLOR").is_some() {
            return false;
        }

        // Windows Terminal explicitly signals ANSI support.
        #[cfg(windows)]
        {
            if std::env::var_os("WT_SESSION").is_some() {
                return true;
            }
        }

        // Default to enabled on every supported platform — modern
        // Windows builds (10.0.10586+) honour ANSI escapes in stderr.
        true
    })
}

impl Default for ConsoleTheme {
    fn default() -> Self {
        if terminal_supports_ansi() {
            Self::with_colors()
        } else {
            Self::plain()
        }
    }
}

impl ConsoleTheme {
    /// Create a new theme with default colors. Auto-detects terminal
    /// color support; falls back to [`Self::plain`] if stderr is not
    /// a TTY, `TERM=dumb`, or `NO_COLOR` is set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new theme with colors forced on, regardless of
    /// terminal detection.
    pub const fn with_colors() -> Self {
        Self {
            error_color: "\x1b[31m",   // Red
            warning_color: "\x1b[33m", // Yellow
            info_color: "\x1b[34m",    // Blue
            success_color: "\x1b[32m", // Green
            caption_color: "\x1b[36m", // Cyan
            reset: "\x1b[0m",
            bold: "\x1b[1m",
            dim: "\x1b[2m",
        }
    }

    /// Create a new theme with no colors (plain text). Useful for
    /// piping output to a file or non-TTY consumer.
    pub const fn plain() -> Self {
        Self {
            error_color: "",
            warning_color: "",
            info_color: "",
            success_color: "",
            caption_color: "",
            reset: "",
            bold: "",
            dim: "",
        }
    }

    /// Format an error message with the error color.
    pub fn error(&self, text: &str) -> String {
        format!("{}{}{}", self.error_color, text, self.reset)
    }

    /// Format a warning message with the warning color.
    pub fn warning(&self, text: &str) -> String {
        format!("{}{}{}", self.warning_color, text, self.reset)
    }

    /// Format an info message with the info color.
    pub fn info(&self, text: &str) -> String {
        format!("{}{}{}", self.info_color, text, self.reset)
    }

    /// Format a success message with the success color.
    pub fn success(&self, text: &str) -> String {
        format!("{}{}{}", self.success_color, text, self.reset)
    }

    /// Format a caption with the caption color.
    pub fn caption(&self, text: &str) -> String {
        format!("{}{}{}", self.caption_color, text, self.reset)
    }

    /// Format text as bold.
    pub fn bold(&self, text: &str) -> String {
        format!("{}{}{}", self.bold, text, self.reset)
    }

    /// Format text as dim.
    pub fn dim(&self, text: &str) -> String {
        format!("{}{}{}", self.dim, text, self.reset)
    }

    /// Format an error display in a structured way.
    ///
    /// Writes the caption, the error's `Display` output, the
    /// retryability marker, and the optional source chain into a
    /// single `String` buffer. Allocates exactly once.
    pub fn format_error<E: crate::error::ForgeError>(&self, err: &E) -> String {
        use std::fmt::Write as _;
        let mut buf = String::with_capacity(160);

        // Caption — written via the helper formatters so the colour
        // escapes match the rest of the output.
        let _ = writeln!(buf, "{}", self.caption(&format!("⚠️  {}", err.caption())));

        // Error message.
        let _ = writeln!(buf, "{}", self.error(&err.to_string()));

        // Retryable status.
        let marker = if err.is_retryable() {
            self.success("Yes")
        } else {
            self.error("No")
        };
        let _ = writeln!(buf, "{}Retryable: {}{}", self.dim, marker, self.reset);

        // Source error if available.
        if let Some(source) = err.source() {
            let _ = writeln!(
                buf,
                "{}Caused by: {}{}",
                self.dim,
                self.error(&source.to_string()),
                self.reset
            );
        }

        buf
    }
}

/// Pretty-print an error to stderr with the default theme.
///
/// The default theme is cached process-wide via `OnceLock` — the
/// terminal-capability check runs at most once regardless of how
/// many errors are printed.
pub fn print_error<E: crate::error::ForgeError>(err: &E) {
    static DEFAULT_THEME: std::sync::OnceLock<ConsoleTheme> = std::sync::OnceLock::new();
    let theme = DEFAULT_THEME.get_or_init(ConsoleTheme::default);
    eprintln!("{}", theme.format_error(err));
}

/// Install a panic hook that formats panics using the ConsoleTheme
pub fn install_panic_hook() {
    let theme = ConsoleTheme::default();
    std::panic::set_hook(Box::new(move |panic_info| {
        let message = match panic_info.payload().downcast_ref::<&str>() {
            Some(s) => *s,
            None => match panic_info.payload().downcast_ref::<String>() {
                Some(s) => s.as_str(),
                None => "Unknown panic",
            },
        };

        let location = if let Some(location) = panic_info.location() {
            format!("at {}:{}", location.file(), location.line())
        } else {
            "at unknown location".to_string()
        };

        eprintln!("{}", theme.caption("💥 PANIC"));
        eprintln!(
            "{}",
            theme.error(&format!("{} {}", message, theme.dim(&location)))
        );
    }));
}
