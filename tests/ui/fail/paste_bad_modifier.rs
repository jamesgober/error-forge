// The internal identifier-pasting macro reports malformed brackets as
// compile errors at the offending token instead of panicking.
error_forge::__private::paste! {
    fn [<Name:snake>]() {}
}

error_forge::__private::paste! {
    fn [<Name::lower>]() {}
}

error_forge::__private::paste! {
    fn [<"text">]() {}
}

error_forge::__private::paste! {
    fn [<Name:>]() {}
}

fn main() {}
