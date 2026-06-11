use serde::{Deserialize, Serialize};

use currencies::{LeaseGroup, PaymentGroup};
use finance::coin::CoinDTO;
use position::ClosePolicyChange;
use remote_lease::callback::RemoteLeaseCallback;
use sdk::cosmwasm_std::Addr;

use self::position::PositionClose;

pub mod authz;
pub mod limits;
pub mod open;
pub mod position;
pub mod query;

pub(crate) type LeasePaymentCurrencies = PaymentGroup;
pub type PaymentCoin = CoinDTO<LeasePaymentCurrencies>;
pub type DownpaymentCoin = PaymentCoin;

// TODO this type is used predominantly in the contract implementation so consider
// deprecating it in favor of crate::finance::PriceG
pub(crate) type LeaseAssetCurrencies = LeaseGroup;
pub type LeaseCoin = CoinDTO<LeaseAssetCurrencies>;

pub type LpnCoinDTO = crate::finance::LpnCoinDTO;

#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct MigrateMsg {}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "skel_testing", derive(Debug))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Repayment
    ///
    /// The funds should be sent attached to the message
    Repay(),

    /// Change the Lease automatic close policy
    ///
    /// The lease owner can set Stop Loss, SL, or/and TakeProfit, TP, triggers after the lease has been fully opened.
    /// They may be set up and removed individually, or together until the lease is fully paid, fully closed, or fully liquidated.
    /// The trigger of SL and TP is defined as LTV percent such as 0 < TP% <= current LTV% < SL% < Liquidation%.
    /// Note that the Liquidation% is set on the Lease Open time and remains intact over the Lease Lifetime.
    ///
    /// A Full Close of the position occurs if:
    /// - SL is set and current LTV% >= SL% , or
    /// - TP is set and TP% > current LTV% .
    ///
    /// The protocol proceeds with a liquidation if the Liquidation% and SL% are surpassed simultaneously.
    ///
    /// The full position close implies that a trigger is consumed and no longer valid once fired.
    ///
    /// It's worth noting that since TP and SL are meant to be triggered on price changes, not past liquidations or payments,
    /// the TP% is reset if a partial liquidation or a payment takes the position LTV below the TP%.
    ChangeClosePolicy(ClosePolicyChange),

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

    /// A callback delivering the outcome of a remote-lease operation
    ///
    /// Invoked by the configured `remote_lease` controller contract after it
    /// receives an IBC ack or timeout for an operation it dispatched on this
    /// lease's behalf. Only the currently-pending dex sub-state of the lease
    /// accepts the callback; it authorises `info.sender == remote_lease`,
    /// classifies the variant, and forwards it through the existing
    /// `on_dex_response` / `on_dex_error` / `on_dex_timeout` pipeline — which
    /// itself enters the `ResponseDelivery` + `DexCallback` safe-delivery
    /// machinery. Synchronous failures (auth mismatch, serialisation,
    /// pre-`ResponseDelivery` storage faults) propagate as `Err` and revert
    /// the controller's `ibc_packet_ack`, letting the relayer retry the same
    /// ack; once `ResponseDelivery` state is persisted the controller's ack
    /// commits and the inner work runs via the lease's own time-alarm
    /// fallback on error.
    RemoteLeaseCallback(RemoteLeaseCallback),

    /// Heal a lease past a middleware failure
    ///
    /// It cures a lease in the following cases:
    /// - on the final repay transaction, when an error, usually an out-of-gas, occurs on the Lpp's ExecuteMsg::RepayLoan sub-message
    /// - on the final repay transaction, when an error occurs on the Lease's SudoMsg::Response message
    Heal(),
}

/// The execute message any `Finalizer` should respond to.
#[derive(Serialize, Deserialize)]
#[cfg_attr(feature = "skel_testing", derive(Debug, PartialEq, Eq))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum FinalizerExecuteMsg {
    FinalizeLease { customer: Addr },
}

#[cfg(all(feature = "internal.test.skel", test))]
mod test {
    use remote_lease::{
        callback::{RemoteErrorMessage, RemoteLeaseCallback},
        response::{CloseLeaseResponse, WireOperationResponse},
    };
    use sdk::cosmwasm_std;

    use crate::api::{
        ExecuteMsg,
        position::{FullClose, PositionClose},
    };

    #[test]
    fn test_remote_lease_callback_timeout_representation() {
        let msg = ExecuteMsg::RemoteLeaseCallback(RemoteLeaseCallback::OperationTimeout);
        let bin = cosmwasm_std::to_json_vec(&msg).expect("serialization failed");
        assert_eq!(
            msg,
            cosmwasm_std::from_json(&bin).expect("deserialization failed"),
        );

        assert_eq!(
            msg,
            cosmwasm_std::from_json("{\"remote_lease_callback\":\"operation_timeout\"}")
                .expect("deserialization failed"),
        );
    }

    #[test]
    fn test_remote_lease_callback_operation_ok_representation() {
        let msg = ExecuteMsg::RemoteLeaseCallback(RemoteLeaseCallback::OperationOk(
            WireOperationResponse::CloseLease(CloseLeaseResponse {}),
        ));
        let bin = cosmwasm_std::to_json_vec(&msg).expect("serialization failed");
        assert_eq!(
            msg,
            cosmwasm_std::from_json(&bin).expect("deserialization failed"),
        );

        assert_eq!(
            msg,
            cosmwasm_std::from_json(
                "{\"remote_lease_callback\":{\"operation_ok\":{\"close_lease\":{}}}}"
            )
            .expect("deserialization failed"),
        );
    }

    #[test]
    fn test_remote_lease_callback_operation_err_representation() {
        let msg = ExecuteMsg::RemoteLeaseCallback(RemoteLeaseCallback::OperationErr(
            RemoteErrorMessage::new("solana side rejected").expect("within length cap"),
        ));
        let bin = cosmwasm_std::to_json_vec(&msg).expect("serialization failed");
        assert_eq!(
            msg,
            cosmwasm_std::from_json(&bin).expect("deserialization failed"),
        );

        assert_eq!(
            msg,
            cosmwasm_std::from_json(
                "{\"remote_lease_callback\":{\"operation_err\":\"solana side rejected\"}}"
            )
            .expect("deserialization failed"),
        );
    }

    #[test]
    fn test_repay_representation() {
        let msg = ExecuteMsg::Repay();
        let repay_bin = cosmwasm_std::to_json_vec(&msg).expect("serialization failed");
        assert_eq!(
            msg,
            cosmwasm_std::from_json(&repay_bin).expect("deserialization failed"),
        );

        assert_eq!(
            msg,
            cosmwasm_std::from_json("{\"repay\":[]}").expect("deserialization failed")
        );
    }

    #[test]
    fn test_close_position_representation() {
        let msg = ExecuteMsg::ClosePosition(PositionClose::FullClose(FullClose {}));
        let close_bin = cosmwasm_std::to_json_vec(&msg).expect("serialization failed");
        assert_eq!(
            msg,
            cosmwasm_std::from_json(&close_bin).expect("deserialization failed"),
        );

        assert_eq!(
            msg,
            cosmwasm_std::from_json("{\"close_position\":{\"full_close\":{}}}")
                .expect("deserialization failed"),
        );
    }
}
