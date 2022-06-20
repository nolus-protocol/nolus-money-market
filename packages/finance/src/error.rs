use std::any::type_name;

use cosmwasm_std::OverflowError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum Error {
    #[error("Programming error or invalid serialized object of {0} type")]
    BrokenInvariant(String),

    #[error("{0}")]
    OverflowError(#[from] OverflowError),

    #[error("Found currency {0} expecting {1}")]
    UnexpectedCurrency(String, String),
}

impl Error {
    pub fn broken_invariant_err<T>() -> Self {
        Self::BrokenInvariant(String::from(type_name::<T>()))
    }
}

pub type Result<T> = core::result::Result<T, Error>;

#[cfg(test)]
mod test {
    use std::any::type_name;

    use super::Error;

    #[test]
    fn broken_invariant_err() {
        enum TestX {}
        let test_x_type_name: &str = type_name::<TestX>();

        let err = Error::broken_invariant_err::<TestX>();
        assert_eq!(
            &Error::BrokenInvariant(test_x_type_name.into()),
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
