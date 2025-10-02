use thiserror::Error;

use finance::error::Error as FinanceError;
use platform::error::Error as PlatformError;
use sdk::cosmwasm_std::StdError;

#[derive(Error, Debug, PartialEq)]
pub enum Error {
    #[error("[Lpp Platform] [Std] {0}")]
    Std(#[from] StdError),

    #[error("[Lpp Platform] [Platform] {0}")]
    Platform(#[from] PlatformError),

    #[error("[Lpp Platform] {0}")]
    Coercion(FinanceError),
}

pub type Result<T> = core::result::Result<T, Error>;
