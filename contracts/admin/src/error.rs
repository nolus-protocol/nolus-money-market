use thiserror::Error;

use finance::currency::SymbolOwned;
use sdk::cosmwasm_std::StdError;

#[derive(Debug, Error)]
pub enum ContractError {
    #[error("[Admin] [Std] {0}")]
    StdError(#[from] StdError),
    #[error("[Admin] {0}")]
    Platform(#[from] platform::error::Error),
    #[error("Migration messages for contracts from group with \"{symbol}\" as a base currency!")]
    MissingMigrationMessages { symbol: SymbolOwned },
    #[error("No data in migration response!")]
    NoMigrationResponseData {},
    #[error("Malformed migration response!")]
    MalformedMigrationResponse(StdError),
    #[error("Contract returned wrong release string! \"{reported}\" was returned, but \"{expected}\" was expected!")]
    WrongRelease { reported: String, expected: String },
}
