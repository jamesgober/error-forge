error_forge::define_errors! {
    pub enum NetError {
        #[kind(Net)]
        Timeout,

        #[error(display = "refused")]
        Refused,
    }
}

fn main() {}
