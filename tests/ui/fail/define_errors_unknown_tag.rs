error_forge::define_errors! {
    pub enum NetError {
        #[kind(Net, retriable = true, status = 503)]
        Timeout,
    }
}

fn main() {}
