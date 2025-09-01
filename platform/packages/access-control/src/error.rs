use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum Error {
    #[error("[Access Control] Unauthorized access!")]
    Unauthorized {},

    #[error("[Access Control] [Std] {0}")]
    Std(String),
}

pub type Result = std::result::Result<(), Error>;
