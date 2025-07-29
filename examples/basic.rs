use std::fmt;

#[derive(Debug)]
pub enum MyError {
    Config { message: String },
    Filesystem { path: String, error: String },
    Crash { reason: String },
}

impl MyError {
    pub fn config(msg: impl Into<String>) -> Self {
        MyError::Config { message: msg.into() }
    }

    pub fn caption(&self) -> &'static str {
        match self {
            Self::Config { .. } => "⚙️ Config",
            Self::Filesystem { .. } => "💾 IO",
            Self::Crash { .. } => "🚨 Panic",
        }
    }

    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::Filesystem { .. })
    }

    pub fn exit_code(&self) -> i32 {
        match self {
            Self::Config { .. } => 78,
            _ => 1,
        }
    }
}


impl fmt::Display for MyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MyError::Config { message } => write!(f, "⚙️ Config: {}", message),
            MyError::Filesystem { path, error } => write!(f, "💾 IO at {}: {}", path, error),
            MyError::Crash { reason } => write!(f, "🚨 Panic: {}", reason),
        }
    }
}

fn fail() -> Result<(), MyError> {
    Err(MyError::config("Missing database URL"))
}

fn main() {
    match fail() {
        Ok(_) => println!("✅ All good!"),
        Err(e) => {
            eprintln!("❌ {}", e);
            eprintln!("↳ Caption: {}", e.caption());
            eprintln!("↳ Retryable: {}", e.is_retryable());
            eprintln!("↳ Exit Code: {}", e.exit_code());
            std::process::exit(e.exit_code());
        }
    }
}