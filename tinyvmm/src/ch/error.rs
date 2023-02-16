use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("database error")]
    Database(#[from] crate::database::error::Error),

    #[error("parse error")]
    Parse(#[from] byte_unit::ByteError),

    #[error("io error")]
    IO(#[from] std::io::Error),

    #[error("serialize error")]
    Serialize(#[from] serde_json::Error),

    #[error("http error")]
    Http(#[from] hyper::http::Error),

    #[error("http error")]
    Hyper(#[from] hyper::Error),

    #[error("http request failed: {0}: `{1}`")]
    HttpNoSuccess(u16, String),

    #[error("utf8 error")]
    UTF8(#[from] std::str::Utf8Error),
}
