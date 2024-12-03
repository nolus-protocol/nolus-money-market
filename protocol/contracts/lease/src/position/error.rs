use std::result::Result as StdResult;

use finance::{error::Error as FinanceError, percent::Percent};
use thiserror::Error;

use crate::finance::LpnCoinDTO;

use super::CloseStrategy;

#[derive(Error, Debug, PartialEq)]
pub enum Error {
    #[error("[Lease] {0}")]
    FinanceError(#[from] FinanceError),

    #[error("[Lease] The asset amount should worth at least {0}")]
    InsufficientAssetAmount(LpnCoinDTO),

    #[error("[Lease] The transaction amount should worth at least {0}")]
    InsufficientTransactionAmount(LpnCoinDTO),

    #[error("[Lease] The position close amount should worth at least {0}")]
    PositionCloseAmountTooSmall(LpnCoinDTO),

    #[error("[Lease] The position past this close should worth at least {0}")]
    PositionCloseAmountTooBig(LpnCoinDTO),

    #[error("[Position] Invalid close policy! The current lease LTV '{lease_ltv}' would trigger '{strategy}'!")]
    TriggerClose {
        lease_ltv: Percent,
        strategy: CloseStrategy,
    },
}

impl Error {
    pub fn trigger_close(lease_ltv: Percent, strategy: CloseStrategy) -> Self {
        Self::TriggerClose {
            lease_ltv,
            strategy,
        }
    }
}

pub type Result<T> = StdResult<T, Error>;
