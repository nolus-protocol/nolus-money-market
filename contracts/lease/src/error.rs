use std::any::type_name;

use cosmwasm_std::{StdError, OverflowError};
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

    #[error("Error in opening an underlying loan: {0}")]
    OpenLoanError(String),

    #[error("{0}")]
    OverflowError(#[from] OverflowError),

    #[error("Custom Error val: {val:?}")]
    CustomError { val: String },
    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
}

impl ContractError {
    pub fn broken_invariant_err<T>() -> Self {
        Self::BrokenInvariant(String::from(type_name::<T>()))
    }
}

pub type ContractResult<T> = core::result::Result<T, ContractError>;

#[cfg(test)]
mod test {
    use std::any::type_name;

    use super::ContractError;

    #[test]
    fn broken_invariant_err() {
        enum TestX {}
        let test_x_type_name: &str = type_name::<TestX>();

        let err = ContractError::broken_invariant_err::<TestX>();
        assert_eq!(
            &ContractError::BrokenInvariant(test_x_type_name.into()),
            &err
        );

        assert_eq!(
            format!(
                "Programming error or invalid serialized object of {} type",
                test_x_type_name
            ),
            format!("{}", err)
        );
    }
}
