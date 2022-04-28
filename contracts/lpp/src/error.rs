use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Unauthorized contract Id")]
    ContractId {},

    #[error("No liquidity")]
    NoLiquidity {},

    #[error("The loan exists")]
    LoanExists {},

    #[error("The loan does not exist")]
    NoLoan {},

    #[error("Lpp requires single denom")]
    FundsLen {},

    #[error("Denoms are different: {contract_denom:?} vs {denom:?}")]
    Denom { contract_denom: String, denom: String},

    #[error("Custom Error val: {val:?}")]
    CustomError { val: String },
    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
}

