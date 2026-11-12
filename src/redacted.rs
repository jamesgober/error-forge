//! A wrapper that keeps a value out of error text.

use std::fmt;

/// A value that prints as `[redacted]` in `Display` and `Debug`.
///
/// Wrap secrets (passwords, tokens, keys) and personal data held in an
/// error, so they never reach `Display`, `Debug`, `dev_message`, the log
/// adapters, or an [`ErrorCollector`](crate::ErrorCollector) summary.
/// The default `define_errors!` format prints every field with `Debug`,
/// so a `Redacted` field prints as `[redacted]` there too. Read the
/// value with [`expose`](Self::expose) where it is really needed.
///
/// `Redacted` only changes how the value is printed. It does not wipe
/// memory and is not a cryptographic protection.
///
/// # Example
///
/// ```
/// use error_forge::Redacted;
///
/// let token = Redacted::new(String::from("hunter2"));
/// assert_eq!(format!("{token}"), "[redacted]");
/// assert_eq!(format!("{token:?}"), "[redacted]");
/// assert_eq!(token.expose(), "hunter2");
/// ```
#[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Redacted<T>(T);

impl<T> Redacted<T> {
    /// Wrap `value`.
    pub const fn new(value: T) -> Self {
        Self(value)
    }

    /// The wrapped value.
    pub const fn expose(&self) -> &T {
        &self.0
    }

    /// Unwrap the value.
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> From<T> for Redacted<T> {
    fn from(value: T) -> Self {
        Self(value)
    }
}

impl<T> fmt::Debug for Redacted<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[redacted]")
    }
}

impl<T> fmt::Display for Redacted<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[redacted]")
    }
}

#[cfg(feature = "serde")]
impl<T> serde::Serialize for Redacted<T> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str("[redacted]")
    }
}

#[cfg(test)]
mod tests {
    use super::Redacted;

    #[test]
    fn never_prints_the_value() {
        let secret = Redacted::from("swordfish");
        assert_eq!(secret.to_string(), "[redacted]");
        assert_eq!(format!("{secret:?}"), "[redacted]");
        assert_eq!(format!("{secret:#?}"), "[redacted]");
        assert_eq!(*secret.expose(), "swordfish");
        assert_eq!(secret.into_inner(), "swordfish");
    }
}
