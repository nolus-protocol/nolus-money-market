use thiserror::Error;

use sdk::cosmwasm_std::StdError;

pub use crate::alarms::{Alarms, AlarmsMut};

mod alarms;
pub mod migrate_v1;

#[derive(Error, Debug, PartialEq)]
pub enum AlarmError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("[Time Oracle] Alarms delivery queue is empty but received reply status")]
    ReplyOnEmptyAlarmQueue,
}
