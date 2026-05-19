//! Wire-format byte-pin tests for the cross-chain `remote_lease` protocol.
//!
//! Acceptance criterion (GH #626): every literal-JSON pin below must equal
//! the JSON the typed `remote_lease` crate emits for the same logical value.
//! The cross-surface integration test under `remote_lease/tests/` validates
//! that equivalence end-to-end; this module locks the wire encoding so the
//! Solana side can rely on a stable surface — **any edit to a literal pin is
//! a breaking protocol change and MUST bump [`crate::VERSION`]**.

use std::fmt::Debug;

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::{
    PORT_PREFIX, VERSION,
    callback::{OPERATION_ERR_MAX_BYTES, RemoteErrorMessage, RemoteLeaseCallback},
    coin::WireCoin,
    envelope::{LeaseAddrOnWire, PacketEnvelope},
    error::Error,
    msg::{CloseLeaseParams, OpenLeaseParams, Operation, SwapParams, TransferOutParams},
    port_id_for,
    response::{
        CloseLeaseResponse, OpenLeaseResponse, OperationResponse, SwapResponse, TransferOutResponse,
    },
    ticker::Ticker,
    version::ProtocolVersion,
};

// ---------------------------------------------------------------------------
// 1. Operation variants — round-trip + literal JSON
// ---------------------------------------------------------------------------

#[test]
fn open_lease_msg_serde() {
    let value = Operation::OpenLease(sample_open_lease_params());
    assert_round_trip_eq(
        r#"{"open_lease":{"expected_instance_ordinal":7,"downpayment_currency":"NLS","lpn_currency":"LPN","asset_currency":"LC1"}}"#,
        &value,
    );
}

#[test]
fn close_lease_msg_serde() {
    let value = Operation::CloseLease(CloseLeaseParams {});
    assert_round_trip_eq(r#"{"close_lease":{}}"#, &value);
}

#[test]
fn swap_msg_serde() {
    let value = Operation::Swap(sample_swap_params());
    assert_round_trip_eq(
        r#"{"swap":{"coin_in":{"amount":"1000","ticker":"NLS"},"min_out":{"amount":"42","ticker":"LPN"}}}"#,
        &value,
    );
}

#[test]
fn transfer_out_msg_serde() {
    let value = Operation::TransferOut(sample_transfer_out_params());
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
        amount_out: WireCoin::new(42, Ticker::new("LPN")),
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
    let value = RemoteLeaseCallback::OperationErr(
        RemoteErrorMessage::new("dex pool drained").expect("short message must be accepted"),
    );
    assert_round_trip_eq(r#"{"operation_err":"dex pool drained"}"#, &value);
}

#[test]
fn callback_operation_timeout_serde() {
    let value = RemoteLeaseCallback::OperationTimeout;
    assert_round_trip_eq(r#""operation_timeout""#, &value);
}

#[test]
fn callback_error_message_at_cap_accepted() {
    let payload = "x".repeat(OPERATION_ERR_MAX_BYTES);
    RemoteErrorMessage::new(payload).expect("payload at the cap must be accepted");
}

#[test]
fn callback_error_message_over_cap_rejected() {
    let payload = "x".repeat(OPERATION_ERR_MAX_BYTES + 1);
    assert!(matches!(
        RemoteErrorMessage::new(payload),
        Err(Error::CallbackErrorTooLong {
            actual,
            max: OPERATION_ERR_MAX_BYTES,
        }) if actual == OPERATION_ERR_MAX_BYTES + 1,
    ));
}

#[test]
fn callback_error_message_deserialize_over_cap_rejected() {
    let payload = "x".repeat(OPERATION_ERR_MAX_BYTES + 1);
    let bad_wire = format!(r#""{payload}""#);
    serde_json::from_str::<RemoteErrorMessage>(&bad_wire)
        .expect_err("over-cap payload must fail deserialization");
}

// ---------------------------------------------------------------------------
// 4. PacketEnvelope — round-trip + literal JSON
// ---------------------------------------------------------------------------

#[test]
fn packet_envelope_serde() {
    let value = PacketEnvelope {
        lease: LeaseAddrOnWire::new("nolus1leaseaddr"),
        operation: Operation::CloseLease(CloseLeaseParams {}),
        version: ProtocolVersion,
    };
    assert_round_trip_eq(
        r#"{"lease":"nolus1leaseaddr","operation":{"close_lease":{}},"version":"nls-remote-lease.v1"}"#,
        &value,
    );
}

#[test]
fn packet_envelope_version_mismatch_rejected() {
    let bad_wire = r#"{"lease":"nolus1leaseaddr","operation":{"close_lease":{}},"version":"nls-remote-lease.v2"}"#;
    serde_json::from_str::<PacketEnvelope>(bad_wire)
        .expect_err("mismatched protocol version must fail deserialization");
}

#[test]
fn packet_envelope_missing_version_rejected() {
    let bad_wire = r#"{"lease":"nolus1leaseaddr","operation":{"close_lease":{}}}"#;
    serde_json::from_str::<PacketEnvelope>(bad_wire)
        .expect_err("missing version field must fail deserialization");
}

#[test]
fn lease_addr_on_wire_round_trip_is_bare_string() {
    let value = LeaseAddrOnWire::new("nolus1leaseaddr");
    assert_round_trip_eq(r#""nolus1leaseaddr""#, &value);
}

// ---------------------------------------------------------------------------
// 5. OpenLeaseParams invariant: three currencies pairwise distinct
// ---------------------------------------------------------------------------

#[test]
fn open_lease_params_distinct_currencies_ok() {
    let params = OpenLeaseParams::new(
        7,
        Ticker::new("NLS"),
        Ticker::new("LPN"),
        Ticker::new("LC1"),
    )
    .expect("three distinct currencies must be accepted");
    assert!(params.invariant_held());
}

#[test]
fn open_lease_params_downpayment_equals_lpn_rejected() {
    let res = OpenLeaseParams::new(
        7,
        Ticker::new("NLS"),
        Ticker::new("NLS"),
        Ticker::new("LC1"),
    );
    assert!(matches!(res, Err(Error::DuplicateLeaseCurrencies)));
}

#[test]
fn open_lease_params_downpayment_equals_asset_rejected() {
    let res = OpenLeaseParams::new(
        7,
        Ticker::new("NLS"),
        Ticker::new("LPN"),
        Ticker::new("NLS"),
    );
    assert!(matches!(res, Err(Error::DuplicateLeaseCurrencies)));
}

#[test]
fn open_lease_params_lpn_equals_asset_rejected() {
    let res = OpenLeaseParams::new(
        7,
        Ticker::new("NLS"),
        Ticker::new("LPN"),
        Ticker::new("LPN"),
    );
    assert!(matches!(res, Err(Error::DuplicateLeaseCurrencies)));
}

#[test]
fn open_lease_params_deserialize_invariant_violation_rejected() {
    let bad_wire = r#"{"expected_instance_ordinal":7,"downpayment_currency":"NLS","lpn_currency":"NLS","asset_currency":"LC1"}"#;
    serde_json::from_str::<OpenLeaseParams>(bad_wire)
        .expect_err("invariant violation must fail deserialization");
}

#[test]
fn open_lease_params_max_ordinal_accepted() {
    let params = OpenLeaseParams::new(
        u16::MAX,
        Ticker::new("NLS"),
        Ticker::new("LPN"),
        Ticker::new("LC1"),
    )
    .expect("u16::MAX is a valid ordinal at the protocol layer");
    assert_eq!(u16::MAX, params.expected_instance_ordinal());
}

#[test]
fn open_lease_params_deserialize_above_u16_rejected() {
    let bad_wire = r#"{"expected_instance_ordinal":65536,"downpayment_currency":"NLS","lpn_currency":"LPN","asset_currency":"LC1"}"#;
    serde_json::from_str::<OpenLeaseParams>(bad_wire)
        .expect_err("ordinal above u16 range must fail deserialization");
}

// ---------------------------------------------------------------------------
// 6. SwapParams invariant: coin_in and min_out currencies differ, both > 0
// ---------------------------------------------------------------------------

#[test]
fn swap_params_distinct_currencies_ok() {
    let params = SwapParams::new(
        WireCoin::new(1000, Ticker::new("NLS")),
        WireCoin::new(42, Ticker::new("LPN")),
    )
    .expect("distinct non-zero amounts must be accepted");
    assert!(params.invariant_held());
}

#[test]
fn swap_params_same_currency_rejected() {
    let res = SwapParams::new(
        WireCoin::new(1000, Ticker::new("NLS")),
        WireCoin::new(42, Ticker::new("NLS")),
    );
    assert!(matches!(res, Err(Error::SameSwapCurrency)));
}

#[test]
fn swap_params_zero_coin_in_rejected() {
    let res = SwapParams::new(
        WireCoin::new(0, Ticker::new("NLS")),
        WireCoin::new(42, Ticker::new("LPN")),
    );
    assert!(matches!(res, Err(Error::ZeroSwapAmount)));
}

#[test]
fn swap_params_zero_min_out_rejected() {
    let res = SwapParams::new(
        WireCoin::new(1000, Ticker::new("NLS")),
        WireCoin::new(0, Ticker::new("LPN")),
    );
    assert!(matches!(res, Err(Error::ZeroSwapAmount)));
}

#[test]
fn swap_params_deserialize_invariant_violation_rejected() {
    let bad_wire =
        r#"{"coin_in":{"amount":"1000","ticker":"NLS"},"min_out":{"amount":"42","ticker":"NLS"}}"#;
    serde_json::from_str::<SwapParams>(bad_wire)
        .expect_err("invariant violation must fail deserialization");
}

#[test]
fn swap_params_deserialize_zero_amount_rejected() {
    let bad_wire =
        r#"{"coin_in":{"amount":"0","ticker":"NLS"},"min_out":{"amount":"42","ticker":"LPN"}}"#;
    serde_json::from_str::<SwapParams>(bad_wire)
        .expect_err("zero coin_in must fail deserialization");
}

// ---------------------------------------------------------------------------
// 7. TransferOutParams invariant: amount > 0
// ---------------------------------------------------------------------------

#[test]
fn transfer_out_params_non_zero_ok() {
    let params = TransferOutParams::new(WireCoin::new(1000, Ticker::new("LC1")))
        .expect("non-zero amount must be accepted");
    assert!(params.invariant_held());
}

#[test]
fn transfer_out_params_zero_rejected() {
    let res = TransferOutParams::new(WireCoin::new(0, Ticker::new("LC1")));
    assert!(matches!(res, Err(Error::ZeroTransferAmount)));
}

#[test]
fn transfer_out_params_deserialize_zero_rejected() {
    let bad_wire = r#"{"amount":{"amount":"0","ticker":"LC1"}}"#;
    serde_json::from_str::<TransferOutParams>(bad_wire)
        .expect_err("zero amount must fail deserialization");
}

// ---------------------------------------------------------------------------
// 8. WireCoin amount validation at deserialise time
// ---------------------------------------------------------------------------

#[test]
fn wire_coin_deserialize_empty_amount_rejected() {
    let bad_wire = r#"{"amount":"","ticker":"NLS"}"#;
    serde_json::from_str::<WireCoin>(bad_wire)
        .expect_err("empty amount must fail deserialization");
}

#[test]
fn wire_coin_deserialize_non_digit_amount_rejected() {
    let bad_wire = r#"{"amount":"12a","ticker":"NLS"}"#;
    serde_json::from_str::<WireCoin>(bad_wire)
        .expect_err("non-digit amount must fail deserialization");
}

#[test]
fn wire_coin_deserialize_signed_amount_rejected() {
    let bad_wire = r#"{"amount":"-1","ticker":"NLS"}"#;
    serde_json::from_str::<WireCoin>(bad_wire)
        .expect_err("signed amount must fail deserialization");
}

#[test]
fn wire_coin_deserialize_leading_zero_rejected() {
    let bad_wire = r#"{"amount":"00","ticker":"NLS"}"#;
    serde_json::from_str::<WireCoin>(bad_wire)
        .expect_err("leading-zero amount must fail deserialization");
}

#[test]
fn wire_coin_deserialize_canonical_zero_accepted() {
    let wire = r#"{"amount":"0","ticker":"NLS"}"#;
    let coin: WireCoin = serde_json::from_str(wire).expect("canonical zero must deserialize");
    assert!(coin.is_zero());
}

// ---------------------------------------------------------------------------
// 9. Wire-protocol constants
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

#[test]
fn protocol_version_round_trip_pinned() {
    assert_round_trip_eq(r#""nls-remote-lease.v1""#, &ProtocolVersion);
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
        Ticker::new("NLS"),
        Ticker::new("LPN"),
        Ticker::new("LC1"),
    )
    .expect("sample uses three distinct currencies")
}

fn sample_swap_params() -> SwapParams {
    SwapParams::new(
        WireCoin::new(1000, Ticker::new("NLS")),
        WireCoin::new(42, Ticker::new("LPN")),
    )
    .expect("sample uses two distinct non-zero amounts")
}

fn sample_transfer_out_params() -> TransferOutParams {
    TransferOutParams::new(WireCoin::new(1000, Ticker::new("LC1")))
        .expect("sample uses a non-zero amount")
}
