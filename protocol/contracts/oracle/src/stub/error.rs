use std::result::Result as StdResult;

use thiserror::Error;

pub type Result<T> = StdResult<T, Error>;

#[derive(Error, Debug, PartialEq)]
pub enum Error {
    #[error("[Oracle] {0}")]
    Finance(#[from] finance::error::Error),
}
