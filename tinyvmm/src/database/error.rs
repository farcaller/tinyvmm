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

    #[error("the key {0} is missing from the source object")]
    MissingKey(&'static str),

    #[error("failed migrating {kind} from {from}{}", to.and_then(|v| Some(format!(" to {v}"))).or_else(|| Some("".into())).unwrap())]
    FailedMigration {
        kind: &'static str,
        from: String,
        to: Option<&'static str>,
    },

    #[error("no migration available for {kind}/{version}")]
    NoMigrationAvailable {
        kind: &'static str,
        version: &'static str,
    },
}
