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
    #[error("[Position] Invalid close policy! The current lease LTV '{lease_ltv}' would trigger a position close due to a take profit at '{take_profit}'!")]
    TriggerTakeProfit {
        lease_ltv: Percent,
        take_profit: Percent,
    },

    #[error("[Position] Invalid close policy! The current lease LTV '{lease_ltv}' would trigger a position close due to a stop loss at '{stop_loss}'!")]
    TriggerStopLoss {
        lease_ltv: Percent,
        stop_loss: Percent,
    },
}

impl Error {
    pub fn trigger_take_profit(lease_ltv: Percent, take_profit: Percent) -> Self {
        Self::TriggerTakeProfit {
            lease_ltv,
            take_profit,
        }
    }

    pub fn trigger_stop_loss(lease_ltv: Percent, stop_loss: Percent) -> Self {
        Self::TriggerStopLoss {
            lease_ltv,
            stop_loss,
        }
    }
}

pub type Result<T> = StdResult<T, Error>;
