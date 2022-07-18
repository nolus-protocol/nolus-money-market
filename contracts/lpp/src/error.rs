use cosmwasm_std::StdError;
use std::num::TryFromIntError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Finance(#[from] finance::error::Error),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Unknown currency")]
    UnknownCurrency {},

    #[error("Unauthorized contract Id")]
    ContractId {},

    #[error("No liquidity")]
    NoLiquidity {},

    #[error("The loan exists")]
    LoanExists {},

    #[error("The loan does not exist")]
    NoLoan {},

    #[error("Lpp requires single currency")]
    FundsLen {},

    #[error("Insufficient balance")]
    InsufficientBalance,

    #[error("{0}")]
    OverflowError(#[from] TryFromIntError),

    #[error("Custom Error val: {val:?}")]
    CustomError { val: String },
    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
}
