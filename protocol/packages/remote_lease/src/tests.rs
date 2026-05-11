//! Wire-format and invariant tests for the cross-chain `remote_lease` protocol.
//!
//! Acceptance criterion (ibc-solray#134): round-trip serde tests for every
//! variant against `cosmwasm_std::to_json_binary` output. The Solana-side
//! consumer is a foreign codebase, so the literal-JSON pins below are part of
//! the wire contract — changing them is a breaking protocol change.

use std::fmt::Debug;

use serde::Serialize;
use serde::de::DeserializeOwned;

use currencies::{
    PaymentGroup,
    testing::{PaymentC1, PaymentC2, PaymentC3},
};
use finance::coin::Coin;

use crate::{
    PORT_PREFIX, VERSION,
    callback::RemoteLeaseCallback,
    envelope::PacketEnvelope,
    error::Error,
    msg::{CloseLeaseParams, LeaseOperationsMsg, OpenLeaseParams, SwapParams, TransferOutParams},
    port_id_for,
    response::{
        CloseLeaseResponse, OpenLeaseResponse, OperationResponse, SwapResponse, TransferOutResponse,
    },
};

// ---------------------------------------------------------------------------
// 1. LeaseOperationsMsg variants — round-trip + literal JSON
// ---------------------------------------------------------------------------

#[test]
fn open_lease_msg_serde() {
    let value = LeaseOperationsMsg::OpenLease(sample_open_lease_params());
    assert_round_trip_eq(
        r#"{"open_lease":{"expected_instance_ordinal":7,"downpayment_currency":"NLS","lpn_currency":"LPN","asset_currency":"LC1"}}"#,
        &value,
    );
}

#[test]
fn close_lease_msg_serde() {
    let value = LeaseOperationsMsg::CloseLease(CloseLeaseParams {});
    assert_round_trip_eq(r#"{"close_lease":{}}"#, &value);
}

#[test]
fn swap_msg_serde() {
    let value = LeaseOperationsMsg::Swap(sample_swap_params());
    assert_round_trip_eq(
        r#"{"swap":{"coin_in":{"amount":"1000","ticker":"NLS"},"min_out":{"amount":"42","ticker":"LPN"}}}"#,
        &value,
    );
}

#[test]
fn transfer_out_msg_serde() {
    let value = LeaseOperationsMsg::TransferOut(TransferOutParams {
        amount: Coin::<PaymentC3>::new(1000).into(),
    });
    assert_round_trip_eq(
        r#"{"transfer_out":{"amount":{"amount":"1000","ticker":"LC1"}}}"#,
        &value,
    );
}

// ---------------------------------------------------------------------------
// 2. OperationResponse variants — round-trip + literal JSON
// ---------------------------------------------------------------------------

#[test]
fn open_lease_response_serde() {
    let value = OperationResponse::OpenLease(OpenLeaseResponse {
        remote_lease_id: "solray-lease-1".to_owned(),
    });
    assert_round_trip_eq(
        r#"{"open_lease":{"remote_lease_id":"solray-lease-1"}}"#,
        &value,
    );
}

#[test]
fn close_lease_response_serde() {
    let value = OperationResponse::CloseLease(CloseLeaseResponse {});
    assert_round_trip_eq(r#"{"close_lease":{}}"#, &value);
}

#[test]
fn swap_response_serde() {
    let value = OperationResponse::Swap(SwapResponse {
        amount_out: Coin::<PaymentC2>::new(42).into(),
    });
    assert_round_trip_eq(
        r#"{"swap":{"amount_out":{"amount":"42","ticker":"LPN"}}}"#,
        &value,
    );
}

#[test]
fn transfer_out_response_serde() {
    let value = OperationResponse::TransferOut(TransferOutResponse {});
    assert_round_trip_eq(r#"{"transfer_out":{}}"#, &value);
}

// ---------------------------------------------------------------------------
// 3. RemoteLeaseCallback variants — round-trip + literal JSON
// ---------------------------------------------------------------------------

#[test]
fn callback_operation_ok_serde() {
    let value =
        RemoteLeaseCallback::OperationOk(OperationResponse::CloseLease(CloseLeaseResponse {}));
    assert_round_trip_eq(r#"{"operation_ok":{"close_lease":{}}}"#, &value);
}

#[test]
fn callback_operation_err_serde() {
    let value = RemoteLeaseCallback::OperationErr("dex pool drained".to_owned());
    assert_round_trip_eq(r#"{"operation_err":"dex pool drained"}"#, &value);
}

#[test]
fn callback_operation_timeout_serde() {
    let value = RemoteLeaseCallback::OperationTimeout;
    assert_round_trip_eq(r#""operation_timeout""#, &value);
}

// ---------------------------------------------------------------------------
// 4. PacketEnvelope — round-trip + literal JSON
// ---------------------------------------------------------------------------

#[test]
fn packet_envelope_serde() {
    let value = PacketEnvelope {
        lease: "nolus1leaseaddr".to_owned(),
        operation: LeaseOperationsMsg::CloseLease(CloseLeaseParams {}),
    };
    assert_round_trip_eq(
        r#"{"lease":"nolus1leaseaddr","operation":{"close_lease":{}}}"#,
        &value,
    );
}

// ---------------------------------------------------------------------------
// 5. OpenLeaseParams invariant: three currencies pairwise distinct
// ---------------------------------------------------------------------------

#[test]
fn open_lease_params_distinct_currencies_ok() {
    let params = OpenLeaseParams::new(
        7,
        currency::dto::<PaymentC1, PaymentGroup>(),
        currency::dto::<PaymentC2, PaymentGroup>(),
        currency::dto::<PaymentC3, PaymentGroup>(),
    )
    .expect("three distinct currencies must be accepted");
    assert!(params.invariant_held());
}

#[test]
fn open_lease_params_downpayment_equals_lpn_rejected() {
    let res = OpenLeaseParams::new(
        7,
        currency::dto::<PaymentC1, PaymentGroup>(),
        currency::dto::<PaymentC1, PaymentGroup>(),
        currency::dto::<PaymentC3, PaymentGroup>(),
    );
    assert!(matches!(res, Err(Error::DuplicateLeaseCurrencies)));
}

#[test]
fn open_lease_params_downpayment_equals_asset_rejected() {
    let res = OpenLeaseParams::new(
        7,
        currency::dto::<PaymentC1, PaymentGroup>(),
        currency::dto::<PaymentC2, PaymentGroup>(),
        currency::dto::<PaymentC1, PaymentGroup>(),
    );
    assert!(matches!(res, Err(Error::DuplicateLeaseCurrencies)));
}

#[test]
fn open_lease_params_lpn_equals_asset_rejected() {
    let res = OpenLeaseParams::new(
        7,
        currency::dto::<PaymentC1, PaymentGroup>(),
        currency::dto::<PaymentC2, PaymentGroup>(),
        currency::dto::<PaymentC2, PaymentGroup>(),
    );
    assert!(matches!(res, Err(Error::DuplicateLeaseCurrencies)));
}

#[test]
fn open_lease_params_deserialize_invariant_violation_rejected() {
    let bad_wire = r#"{"expected_instance_ordinal":7,"downpayment_currency":"NLS","lpn_currency":"NLS","asset_currency":"LC1"}"#;
    serde_json::from_str::<OpenLeaseParams>(bad_wire)
        .expect_err("invariant violation must fail deserialization");
}

// ---------------------------------------------------------------------------
// 6. SwapParams invariant: coin_in and min_out currencies differ
// ---------------------------------------------------------------------------

#[test]
fn swap_params_distinct_currencies_ok() {
    let params = SwapParams::new(
        Coin::<PaymentC1>::new(1000).into(),
        Coin::<PaymentC2>::new(42).into(),
    )
    .expect("distinct currencies must be accepted");
    assert!(params.invariant_held());
}

#[test]
fn swap_params_same_currency_rejected() {
    let res = SwapParams::new(
        Coin::<PaymentC1>::new(1000).into(),
        Coin::<PaymentC1>::new(42).into(),
    );
    assert!(matches!(res, Err(Error::SameSwapCurrency)));
}

#[test]
fn swap_params_deserialize_invariant_violation_rejected() {
    let bad_wire =
        r#"{"coin_in":{"amount":"1000","ticker":"NLS"},"min_out":{"amount":"42","ticker":"NLS"}}"#;
    serde_json::from_str::<SwapParams>(bad_wire)
        .expect_err("invariant violation must fail deserialization");
}

// ---------------------------------------------------------------------------
// 7. Wire-protocol constants
// ---------------------------------------------------------------------------

#[test]
fn version_constant_pinned() {
    assert_eq!("nls-remote-lease.v1", VERSION);
}

#[test]
fn port_prefix_constant_pinned() {
    assert_eq!("nls-remote-lease.", PORT_PREFIX);
}

#[test]
fn port_id_for_dex_concatenates_prefix() {
    assert_eq!("nls-remote-lease.astroport", port_id_for("astroport"));
}

// ---------------------------------------------------------------------------
// helpers — expected value first per project rule 17
// ---------------------------------------------------------------------------

fn assert_round_trip_eq<T>(expected_json: &str, value: &T)
where
    T: Serialize + DeserializeOwned + PartialEq + Debug,
{
    let encoded = serde_json::to_string(value).expect("serialization must succeed");
    assert_eq!(expected_json, encoded.as_str());

    let decoded: T =
        serde_json::from_str(&encoded).expect("decoding the freshly-encoded value must succeed");
    assert_eq!(value, &decoded);
}

fn sample_open_lease_params() -> OpenLeaseParams {
    OpenLeaseParams::new(
        7,
        currency::dto::<PaymentC1, PaymentGroup>(),
        currency::dto::<PaymentC2, PaymentGroup>(),
        currency::dto::<PaymentC3, PaymentGroup>(),
    )
    .expect("sample uses three distinct currencies")
}

fn sample_swap_params() -> SwapParams {
    SwapParams::new(
        Coin::<PaymentC1>::new(1000).into(),
        Coin::<PaymentC2>::new(42).into(),
    )
    .expect("sample uses two distinct currencies")
}
