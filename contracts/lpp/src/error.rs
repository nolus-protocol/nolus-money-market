use cosmwasm_std::StdError;
use std::num::TryFromIntError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Finance(#[from] finance::error::Error),

    #[error("{0}")]
    Platform(#[from] platform::error::Error),

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

    #[error("The deposit does not exist")]
    NoDeposit {},

    #[error("Zero loan amount")]
    ZeroLoanAmount,

    #[error("Zero deposit")]
    ZeroDepositFunds,

    #[error("Zero withdraw amount")]
    ZeroWithdrawFunds,

    #[error("No pending rewards")]
    NoRewards {},

    #[error("Zero rewards")]
    ZeroRewardsFunds {},

    #[error("Distribute rewards with zero balance nlpn")]
    ZeroBalanceRewards {},

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
