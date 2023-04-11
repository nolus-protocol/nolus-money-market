use thiserror::Error;

use sdk::cosmwasm_std::StdError;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("[Lpp] [Std] {0}")]
    Std(#[from] StdError),

    #[error("[Lpp] {0}")]
    Finance(#[from] finance::error::Error),

    #[error("[Lpp] {0}")]
    Platform(#[from] platform::error::Error),

    #[error("[Lpp] {0}")]
    Unauthorized(#[from] access_control::Unauthorized),

    #[error("[Lpp] Invalid config parameter! {0}")]
    InvalidConfigParameter(&'static str),

    #[error("[Lpp] Unknown currency")]
    UnknownCurrency {},

    #[error("[Lpp] No liquidity")]
    NoLiquidity {},

    #[error("[Lpp] The loan exists")]
    LoanExists {},

    #[error("[Lpp] The loan does not exist")]
    NoLoan {},

    #[error("[Lpp] The deposit does not exist")]
    NoDeposit {},

    #[error("[Lpp] Zero loan amount")]
    ZeroLoanAmount,

    #[error("[Lpp] Zero deposit")]
    ZeroDepositFunds,

    #[error("[Lpp] Zero withdraw amount")]
    ZeroWithdrawFunds,

    #[error("[Lpp] No pending rewards")]
    NoRewards {},

    #[error("[Lpp] Zero rewards")]
    ZeroRewardsFunds {},

    #[error("[Lpp] Distribute rewards with zero balance nlpn")]
    ZeroBalanceRewards {},

    #[error("[Lpp] Lpp requires single currency")]
    FundsLen {},

    #[error("[Lpp] Insufficient balance")]
    InsufficientBalance,

    #[error("[Lpp] Balance overflow")]
    OverflowError,

    #[error("[Lpp] Custom Error val: {val:?}")]
    CustomError { val: String },
    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
}

pub type Result<T> = std::result::Result<T, ContractError>;
