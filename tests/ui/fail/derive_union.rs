use error_forge::ModError;

#[derive(ModError)]
pub union Bits {
    a: u32,
    b: f32,
}

fn main() {}
