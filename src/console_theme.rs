//! Console theming for error display in CLI applications
//! 
//! This module provides ANSI color formatting for error messages
//! displayed in terminal environments.

/// Color theme for console error output
pub struct ConsoleTheme {
    error_color: String,
    warning_color: String,
    info_color: String,
    success_color: String,
    caption_color: String,
    reset: String,
    bold: String,
    dim: String,
}

impl Default for ConsoleTheme {
    fn default() -> Self {
        Self {
            error_color: "\x1b[31m".to_string(),   // Red
            warning_color: "\x1b[33m".to_string(), // Yellow
            info_color: "\x1b[34m".to_string(),    // Blue
            success_color: "\x1b[32m".to_string(), // Green
            caption_color: "\x1b[36m".to_string(), // Cyan
            reset: "\x1b[0m".to_string(),
            bold: "\x1b[1m".to_string(),
            dim: "\x1b[2m".to_string(),
        }
    }
}

impl ConsoleTheme {
    /// Create a new theme with default colors
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Create a new theme with no colors (plain text)
    pub fn plain() -> Self {
        Self {
            error_color: "".to_string(),
            warning_color: "".to_string(),
            info_color: "".to_string(),
            success_color: "".to_string(),
            caption_color: "".to_string(),
            reset: "".to_string(),
            bold: "".to_string(),
            dim: "".to_string(),
        }
    }
    
    /// Format an error message with the error color
    pub fn error(&self, text: &str) -> String {
        format!("{}{}{}", self.error_color, text, self.reset)
    }
    
    /// Format a warning message with the warning color
    pub fn warning(&self, text: &str) -> String {
        format!("{}{}{}", self.warning_color, text, self.reset)
    }
    
    /// Format an info message with the info color
    pub fn info(&self, text: &str) -> String {
        format!("{}{}{}", self.info_color, text, self.reset)
    }
    
    /// Format a success message with the success color
    pub fn success(&self, text: &str) -> String {
        format!("{}{}{}", self.success_color, text, self.reset)
    }
    
    /// Format a caption with the caption color
    pub fn caption(&self, text: &str) -> String {
        format!("{}{}{}", self.caption_color, text, self.reset)
    }
    
    /// Format text as bold
    pub fn bold(&self, text: &str) -> String {
        format!("{}{}{}", self.bold, text, self.reset)
    }
    
    /// Format text as dim
    pub fn dim(&self, text: &str) -> String {
        format!("{}{}{}", self.dim, text, self.reset)
    }
    
    /// Format an error display in a structured way
    pub fn format_error<E: crate::error::ForgeError>(&self, err: &E) -> String {
        let mut result = String::new();
        
        // Add the error caption
        result.push_str(&format!("{}\n", self.caption(&format!("⚠️  {}", err.caption()))));
        
        // Add the error message
        result.push_str(&format!("{}\n", self.error(&err.to_string())));
        
        // Add retryable status if applicable
        if err.is_retryable() {
            result.push_str(&format!("{}Retryable: {}{}\n", 
                self.dim, 
                self.success("Yes"), 
                self.reset
            ));
        } else {
            result.push_str(&format!("{}Retryable: {}{}\n", 
                self.dim, 
                self.error("No"), 
                self.reset
            ));
        }
        
        // Add source error if available
        if let Some(source) = err.source() {
            result.push_str(&format!("{}Caused by: {}{}\n", 
                self.dim, 
                self.error(&source.to_string()), 
                self.reset
            ));
        }
        
        result
    }
}

/// Pretty-print an error to stderr with the default theme
pub fn print_error<E: crate::error::ForgeError>(err: &E) {
    let theme = ConsoleTheme::default();
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
        eprintln!("{}", theme.error(&format!("{} {}", message, theme.dim(&location))));
    }));
}
