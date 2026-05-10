pub mod domain;
pub mod error;

pub use error::PylosError;
pub type Result<T> = std::result::Result<T, PylosError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = PylosError::NotFound("something".into());
        assert_eq!(format!("{}", err), "Not found: something");
    }
}
