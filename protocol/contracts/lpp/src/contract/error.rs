use std::fmt::{Debug, Display};

use thiserror::Error;

use sdk::cosmwasm_std::StdError;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("[Lpp] [Std] {0}")]
    Std(#[from] StdError),

    #[error("[Lpp] Failed to convert query response to binary! Cause: {0}")]
    ConvertToBinary(StdError),

    #[error("[Lpp] {0}")]
    Currency(#[from] currency::error::Error),

    #[error("[Lpp] Invalid Oracle Base currency! Cause: {0}")]
    InvalidOracleBaseCurrency(oracle_platform::error::Error),

    #[error("[Lpp] Failure converting from the quote currency! Cause: {0}")]
    ConvertFromQuote(oracle::stub::Error),

    #[error("[Lpp] {0}")]
    Finance(#[from] finance::error::Error),

    #[error("[Lpp] {0}")]
    Platform(#[from] platform::error::Error),

    #[error("[Lpp] Failed to update software! Cause: {0}")]
    UpdateSoftware(versioning::Error),

    #[error("[Lpp] {0}")]
    Unauthorized(#[from] access_control::error::Error),

    #[error("[Lpp] Unknown currency, details '{0}'")]
    UnknownCurrency(currency::error::Error),

    #[error("[Lpp] No liquidity")]
    NoLiquidity {},

    #[error("[Lpp] The loan does not exist")]
    NoLoan {},

    #[error("[Lpp] The loan exists")]
    LoanExists {},

    #[error("[Lpp] The deposit does not exist")]
    NoDeposit {},

    #[error("[Lpp] Zero loan amount")]
    ZeroLoanAmount,

    #[error("[Lpp] Zero deposit amount")]
    ZeroDepositFunds,

    #[error(
        "[Lpp] Insufficient deposit amount! It must at least be total to the smallest receipt unit!"
    )]
    DepositLessThanAReceipt,

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

    #[error("[Lpp] Utilization is below the set minimal rate")]
    UtilizationBelowMinimalRates,

    #[error("[Lpp Stub] No response sent back from LPP contract")]
    NoResponseStubError,

    #[error("[Lpp] Computation overflow during '{cause}`. `{details}`")]
    ComputationOverflow {
        cause: &'static str,
        details: String,
    },
}

pub type Result<T> = std::result::Result<T, ContractError>;

impl ContractError {
    pub fn overflow_price_total<C, P>(cause: &'static str, amount: C, price: P) -> Self
    where
        C: Display,
        P: Debug,
    {
        Self::computation_overflow(cause, format!("Amount: {}, price: {:?}", amount, price))
    }

    pub fn overflow_add<L, R>(cause: &'static str, lhs: L, rhs: R) -> Self
    where
        L: Display,
        R: Display,
    {
        Self::computation_overflow(cause, format!("({} + {})", lhs, rhs))
    }

    pub fn overflow_sub<L, R>(cause: &'static str, lhs: L, rhs: R) -> Self
    where
        L: Display,
        R: Display,
    {
        Self::computation_overflow(cause, format!("({} - {})", lhs, rhs))
    }

    pub fn overflow_loan_repayment<T, A>(cause: &'static str, timestamp: T, repay_amount: A) -> Self
    where
        T: Display,
        A: Display,
    {
        Self::computation_overflow(
            cause,
            format!("repay amount: {}, timestamp: {}", repay_amount, timestamp),
        )
    }

    pub fn overflow_register_repayment<T, L, P>(
        cause: &'static str,
        timestamp: T,
        loan: L,
        payment: P,
    ) -> Self
    where
        T: Display,
        L: Debug,
        P: Debug,
    {
        Self::computation_overflow(
            cause,
            format!(
                "timestamp: {}, loan: {:?}, payment: {:?}",
                timestamp, loan, payment
            ),
        )
    }

    pub fn overflow_total_due<T>(cause: &'static str, timestamp: T) -> Self
    where
        T: Display,
    {
        Self::computation_overflow(cause, format!("timestamp: {}", timestamp))
    }

    pub fn overflow_total_interest_due_by_now<T>(cause: &'static str, timestamp: T) -> Self
    where
        T: Display,
    {
        Self::computation_overflow(cause, format!("timestamp: {}", timestamp))
    }

    fn computation_overflow(cause: &'static str, details: String) -> Self {
        Self::ComputationOverflow { cause, details }
    }
}
