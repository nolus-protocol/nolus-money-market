use serde::{Deserialize, Serialize};

pub(crate) use currencies::Lpn as LpnCurrency;
use currencies::{LeaseGroup, Lpns, PaymentGroup};
use finance::coin::{Coin, CoinDTO};
use sdk::{
    cosmwasm_std::Addr,
    schemars::{self, JsonSchema},
};

use self::position::PositionClose;

pub mod open;
pub mod position;
pub mod query;

pub(crate) type LeasePaymentCurrencies = PaymentGroup;
pub type PaymentCoin = CoinDTO<LeasePaymentCurrencies>;
pub type DownpaymentCoin = PaymentCoin;

pub(crate) type LeaseAssetCurrencies = LeaseGroup;
pub type LeaseCoin = CoinDTO<LeaseAssetCurrencies>;

pub(crate) type LpnCurrencies = Lpns;
pub type LpnCoinDTO = CoinDTO<LpnCurrencies>;
pub type LpnCoin = Coin<LpnCurrency>;

#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct MigrateMsg {}

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
    ///
    /// Note that these checks would not be performed on the total position amount if
    /// a `PositionClose::FullClose` is requested. It is executed irrespective of the amount.
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

#[cfg(test)]
mod test {
    use sdk::{
        cosmwasm_std::{from_json, to_json_vec},
        schemars::_serde_json::to_string,
    };

    use crate::api::{
        position::{FullClose, PositionClose},
        ExecuteMsg,
    };

    #[test]
    fn test_repay_representation() {
        let msg = ExecuteMsg::Repay();
        let repay_bin = to_json_vec(&msg).expect("serialization failed");
        assert_eq!(
            from_json::<ExecuteMsg>(&repay_bin).expect("deserialization failed"),
            msg
        );

        assert_eq!(
            to_string(&msg).expect("deserialization failed"),
            r#"{"repay":[]}"#
        );
    }

    #[test]
    fn test_close_position_representation() {
        let msg = ExecuteMsg::ClosePosition(PositionClose::FullClose(FullClose {}));
        let close_bin = to_json_vec(&msg).expect("serialization failed");
        assert_eq!(
            from_json::<ExecuteMsg>(&close_bin).expect("deserialization failed"),
            msg
        );

        assert_eq!(
            to_string(&msg).expect("deserialization failed"),
            r#"{"close_position":{"full_close":{}}}"#
        );
    }
}
