error_forge::define_errors! {
    pub enum NetError {
        #[error(display = "first")]
        #[kind(Net)]
        #[error(display = "second")]
        Timeout,
    }
}

fn main() {}
