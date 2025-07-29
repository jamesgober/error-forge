#[cfg(test)]

mod tests {
    use error_forge::error::AppError;

    #[test]
    fn test_error_display() {
        let e = AppError::Other { message: "Boom".into() };
        assert_eq!(format!("{}", e), "🚨 Error: Other | message = \"Boom\"");

        let e = AppError::Config { message: "Missing path".into() };
        assert_eq!(format!("{}", e), "⚙️ Config: Config | message = \"Missing path\"");

    }
}
