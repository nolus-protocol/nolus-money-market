use std::any::type_name;

use cosmwasm_std::StdError;
use cw_utils::PaymentError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Payment error: {0}")]
    PaymentError(#[from] PaymentError),

    #[error("Programming error or invalid serialized object of {0} type")]
    BrokenInvariant(String),

    #[error("Custom Error val: {val:?}")]
    CustomError { val: String },
    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
}

impl ContractError {
    pub fn broken_invariant_err<T>() -> Self {
        Self::BrokenInvariant(String::from(
            type_name::<T>(),
        ))
    }
}
