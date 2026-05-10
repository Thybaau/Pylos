pub mod error;

pub use error::PylosError;
pub type Result<T> = std::result::Result<T, PylosError>;
