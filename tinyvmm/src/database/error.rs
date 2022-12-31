use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("serialize error: {0}")]
    Serialize(#[from] serde_json::Error),

    #[error("top level type is not an object")]
    NotAnObject,

    #[error("serialize validation error: {0}")]
    SerializeValidation(#[from] serde_valid::Error<serde_json::Error>),
}
