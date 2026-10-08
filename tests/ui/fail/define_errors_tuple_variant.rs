error_forge::define_errors! {
    pub enum NetError {
        #[kind(Net)]
        Timeout(u32),
    }
}

fn main() {}
