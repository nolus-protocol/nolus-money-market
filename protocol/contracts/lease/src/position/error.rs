use std::result::Result as StdResult;

use finance::percent::Percent;
use thiserror::Error;

use super::CloseStrategy;

#[derive(Error, Debug, PartialEq)]
pub enum Error {
    // TODO move the following definitions from the lease ContractError down here
    // #[error("[Lease] No payment sent")]
    // NoPaymentError(),

    // #[error("[Lease] Insufficient payment amount {0}")]
    // InsufficientPayment(PaymentCoin),

    // #[error("[Lease] The asset amount should worth at least {0}")]
    // InsufficientAssetAmount(LpnCoinDTO),

    // #[error("[Lease] The transaction amount should worth at least {0}")]
    // InsufficientTransactionAmount(LpnCoinDTO),

    // #[error("[Lease] The position close amount should worth at least {0}")]
    // PositionCloseAmountTooSmall(LpnCoinDTO),

    // #[error("[Lease] The position past this close should worth at least {0}")]
    // PositionCloseAmountTooBig(LpnCoinDTO),
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
