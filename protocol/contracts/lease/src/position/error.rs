use std::result::Result as StdResult;

use finance::percent::Percent;
use thiserror::Error;

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
    #[error("[Position] Invalid close policy! Take profit percent '{take_profit}' should not be higher than the stop loss percent '{stop_loss}'!")]
    InvalidClosePolicy {
        stop_loss: Percent,
        take_profit: Percent,
    },
}

impl Error {
    pub fn invalid_close_policy(stop_loss: Percent, take_profit: Percent) -> Self {
        Self::InvalidClosePolicy {
            stop_loss,
            take_profit,
        }
    }
}

pub type Result<T> = StdResult<T, Error>;
