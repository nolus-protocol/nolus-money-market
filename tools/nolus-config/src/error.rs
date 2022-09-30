#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("I/O error occurred! Context: {0}")]
    IO(#[from] std::io::Error),
    #[error("Serialization error occurred! Context: {0}")]
    Serialization(String),
    #[error("Deserialization error occurred! Context: {0}")]
    Deserialization(String),
}

impl Error {
    pub fn from_serialization<E>(error: E) -> Self
    where
        E: serde::ser::Error,
    {
        Self::Serialization(format!("{error}"))
    }

    pub fn from_deserialization<E>(error: E) -> Self
    where
        E: serde::de::Error,
    {
        Self::Deserialization(format!("{error}"))
    }
}
