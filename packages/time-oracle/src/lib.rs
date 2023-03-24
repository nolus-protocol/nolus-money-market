use sdk::cosmwasm_std::StdError;
use thiserror::Error;

pub use crate::alarms::Alarms;

mod alarms;

#[derive(Error, Debug, PartialEq)]
pub enum AlarmError {
    #[error("{0}")]
    Std(#[from] StdError),
}
