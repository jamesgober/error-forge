use error_forge::ModError;

#[derive(Debug, ModError)]
pub enum BadError {
    #[error_http_status("404")]
    #[error_exit_code(99999999999)]
    #[error_retryable("yes")]
    #[error_display(42)]
    A,

    #[error_http_status(-1)]
    B,
}

fn main() {}
