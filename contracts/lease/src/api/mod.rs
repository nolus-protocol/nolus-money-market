use serde::{Deserialize, Serialize};

use currency::{lease::LeaseGroup, payment::PaymentGroup};
use finance::coin::CoinDTO;
use lpp::msg::LpnCoin as LppLpnCoin;
use sdk::{
    cosmwasm_std::Addr,
    schemars::{self, JsonSchema},
};

pub use self::{
    open::{
        ConnectionParams, Ics20Channel, InterestPaymentSpec, LoanForm, NewLeaseContract,
        NewLeaseForm, PositionSpec,
    },
    position::{FullClose, PartialClose, PositionClose},
    query::{opened, opening, paid, StateQuery, StateResponse},
};

// TODO consider defining the modules public instead of just selected items
mod open;
mod position;
mod query;

pub type PaymentCoin = CoinDTO<PaymentGroup>;
pub type DownpaymentCoin = PaymentCoin;
pub type LeaseCoin = CoinDTO<LeaseGroup>;
pub type LpnCoin = LppLpnCoin;

#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct MigrateMsg {
    pub(crate) customer: Addr,
    pub(crate) finalizer: Addr,
}
impl MigrateMsg {
    pub fn new(customer: Addr, finalizer: Addr) -> Self {
        Self {
            customer,
            finalizer,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Repayment
    ///
    /// The funds should be sent attached to the message
    Repay(),

    /// Customer initiated position close
    ///
    /// Return `error::ContractError::PositionCloseAmountTooSmall` when a partial close is requested
    /// with amount less than the minimum sell asset position parameter sent on lease open. Refer to
    /// `NewLeaseForm::position_spec`.
    ///
    /// Return `error::ContractError::PositionCloseAmountTooBig` when a partial close is requested
    /// with amount that would decrease a position less than the minimum asset parameter sent on
    /// lease open. Refer to `NewLeaseForm::position_spec`.
    ClosePosition(PositionClose),

    /// Close of a fully paid lease
    Close(),

    PriceAlarm(),
    TimeAlarm {},

    /// An entry point for safe delivery of a Dex response
    ///
    /// Invoked always by the same contract instance.
    DexCallback(),

    /// An entry point for safe delivery of an ICA open response, error or timeout
    ///
    /// Invoked always by the same contract instance.
    DexCallbackContinue(),

    /// Heal a lease past a middleware failure
    ///
    /// It cures a lease in the following cases:
    /// - on the final repay transaction, when an error, usually an out-of-gas, occurs on the Lpp's ExecuteMsg::RepayLoan sub-message
    /// - on the final repay transaction, when an error occurs on the Lease's SudoMsg::Response message
    Heal(),
}

/// The execute message any `Finalizer` should respond to.
#[derive(Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug, PartialEq, Eq))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum FinalizerExecuteMsg {
    FinalizeLease { customer: Addr },
}
