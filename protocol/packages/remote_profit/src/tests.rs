//! Wire-format and invariant tests for the cross-chain `remote_profit`
//! protocol.
//!
//! Acceptance criterion (ibc-solray#134): round-trip serde tests for every
//! variant against `cosmwasm_std::to_json_binary` output. The Solana-side
//! consumer is a foreign codebase, so every literal-JSON pin below is part of
//! the wire contract — **any edit to a literal pin is a breaking protocol
//! change and MUST bump [`crate::VERSION`]**, with one exception: an additive
//! field marked `#[serde(default)]` (e.g. `nonce`, #636) extends the wire
//! without a version bump, because updated consumers decode both the old and
//! new shapes and the rollout is coordinated consumer-first rather than
//! signalled by the version.

use std::fmt::Debug;

use serde::Serialize;
use serde::de::DeserializeOwned;

use currencies::testing::{PaymentC1, PaymentC2, PaymentC3};
use finance::coin::Coin;
use remote_profit_wire::nolus_receiver::NolusReceiver;

use crate::{
    PORT_PREFIX, VERSION,
    callback::{
        OPERATION_ERR_MAX_BYTES, RemoteErrorMessage, RemoteOperationOutcome, RemoteProfitCallback,
    },
    envelope::PacketEnvelope,
    error::Error,
    msg::{CloseProfitParams, OpenProfitParams, Operation, SwapParams, TransferOutParams},
    port_id_for,
    response::{
        CloseProfitResponse, OpenProfitResponse, OperationResponse, RemoteProfitId, SwapResponse,
        TransferOutResponse, WireOperationResponse,
    },
    version::ProtocolVersion,
};

// ---------------------------------------------------------------------------
// 1. Operation variants — round-trip + literal JSON
// ---------------------------------------------------------------------------

#[test]
fn open_profit_msg_serde() {
    let value = Operation::OpenProfit(sample_open_profit_params());
    assert_round_trip_eq(
        r#"{"open_profit":{"expected_instance_ordinal":7,"nolus_receiver":"nolus1mf6ptkssddfmxvhdx0ech0k03ktp6kf9yk59renau2gvht3nq2gqkxgywu"}}"#,
        &value,
    );
}

#[test]
fn close_profit_msg_serde() {
    let value = Operation::CloseProfit(CloseProfitParams {});
    assert_round_trip_eq(r#"{"close_profit":{}}"#, &value);
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
fn open_profit_response_serde() {
    let value = OperationResponse::OpenProfit(OpenProfitResponse {
        remote_profit_id: RemoteProfitId::new("So1RayProfit").expect("base58 profit id"),
    });
    assert_round_trip_eq(
        r#"{"open_profit":{"remote_profit_id":"So1RayProfit"}}"#,
        &value,
    );
}

#[test]
fn close_profit_response_serde() {
    let value = OperationResponse::CloseProfit(CloseProfitResponse {});
    assert_round_trip_eq(r#"{"close_profit":{}}"#, &value);
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
// 3. RemoteProfitCallback variants — round-trip + literal JSON
// ---------------------------------------------------------------------------

#[test]
fn callback_operation_ok_serde() {
    let value = RemoteOperationOutcome::OperationOk(WireOperationResponse::CloseProfit(
        CloseProfitResponse {},
    ));
    assert_round_trip_eq(r#"{"operation_ok":{"close_profit":{}}}"#, &value);
}

#[test]
fn callback_operation_err_serde() {
    let value = RemoteOperationOutcome::OperationErr(
        RemoteErrorMessage::new("dex pool drained").expect("short message must be accepted"),
    );
    assert_round_trip_eq(r#"{"operation_err":"dex pool drained"}"#, &value);
}

#[test]
fn callback_operation_timeout_serde() {
    let value = RemoteOperationOutcome::OperationTimeout;
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

// AC (#636): the typed callback carries the per-emission `nonce` alongside the
// outcome and round-trips byte-identically to the wire shape the controller
// dispatches into the profit instance.
#[test]
fn callback_round_trips_with_nonce() {
    let value = RemoteProfitCallback {
        nonce: 7,
        outcome: RemoteOperationOutcome::OperationOk(WireOperationResponse::CloseProfit(
            CloseProfitResponse {},
        )),
    };
    assert_round_trip_eq(
        r#"{"nonce":7,"outcome":{"operation_ok":{"close_profit":{}}}}"#,
        &value,
    );
}

// ---------------------------------------------------------------------------
// 4. PacketEnvelope — round-trip + literal JSON
// ---------------------------------------------------------------------------

#[test]
fn packet_envelope_serde() {
    let value = PacketEnvelope {
        operation: Operation::CloseProfit(CloseProfitParams {}),
        version: ProtocolVersion,
        nonce: 0,
    };
    assert_round_trip_eq(
        r#"{"operation":{"close_profit":{}},"version":"nls-remote-profit.v1","nonce":0}"#,
        &value,
    );
}

#[test]
fn packet_envelope_version_mismatch_rejected() {
    let bad_wire = r#"{"operation":{"close_profit":{}},"version":"nls-remote-profit.v2"}"#;
    serde_json::from_str::<PacketEnvelope>(bad_wire)
        .expect_err("mismatched protocol version must fail deserialization");
}

#[test]
fn packet_envelope_missing_version_rejected() {
    let bad_wire = r#"{"operation":{"close_profit":{}}}"#;
    serde_json::from_str::<PacketEnvelope>(bad_wire)
        .expect_err("missing version field must fail deserialization");
}

// AC (#636): the typed envelope carries `nonce` as its last field (after
// `version`) and a non-zero nonce round-trips byte-identically to the wire
// JSON the Solana side consumes.
#[test]
fn packet_envelope_round_trips_with_nonce() {
    let value = PacketEnvelope {
        operation: Operation::CloseProfit(CloseProfitParams {}),
        version: ProtocolVersion,
        nonce: 7,
    };
    assert_round_trip_eq(
        r#"{"operation":{"close_profit":{}},"version":"nls-remote-profit.v1","nonce":7}"#,
        &value,
    );
}

// AC (#636): `nonce` is `#[serde(default)]`, so a packet that predates the
// field decodes with `nonce == 0` instead of being rejected.
#[test]
fn packet_envelope_decodes_without_nonce_to_zero() {
    let wire = r#"{"operation":{"close_profit":{}},"version":"nls-remote-profit.v1"}"#;
    let envelope: PacketEnvelope =
        serde_json::from_str(wire).expect("a payload without nonce must default it to zero");
    assert_eq!(0, envelope.nonce);
}

// ---------------------------------------------------------------------------
// 5. OpenProfitParams: singleton establishment, ordinal replay guard only
// ---------------------------------------------------------------------------

#[test]
fn open_profit_params_round_trips_with_nolus_receiver() {
    let value = OpenProfitParams::new(7, sample_nolus_receiver());
    assert_round_trip_eq(
        r#"{"expected_instance_ordinal":7,"nolus_receiver":"nolus1mf6ptkssddfmxvhdx0ech0k03ktp6kf9yk59renau2gvht3nq2gqkxgywu"}"#,
        &value,
    );
    assert_eq!(SAMPLE_NOLUS_RECEIVER, value.nolus_receiver().as_str());
}

#[test]
fn open_profit_params_max_ordinal_accepted() {
    let params = OpenProfitParams::new(u16::MAX, sample_nolus_receiver());
    assert_eq!(u16::MAX, params.expected_instance_ordinal());
}

#[test]
fn open_profit_params_deserialize_above_u16_rejected() {
    let bad_wire = r#"{"expected_instance_ordinal":65536,"nolus_receiver":"nolus1mf6ptkssddfmxvhdx0ech0k03ktp6kf9yk59renau2gvht3nq2gqkxgywu"}"#;
    serde_json::from_str::<OpenProfitParams>(bad_wire)
        .expect_err("ordinal above u16 range must fail deserialization");
}

// `nolus_receiver` is a plain required field (no `#[serde(default)]`): the old
// single-field shape must be rejected outright.
#[test]
fn open_profit_params_rejects_missing_nolus_receiver() {
    let bad_wire = r#"{"expected_instance_ordinal":7}"#;
    serde_json::from_str::<OpenProfitParams>(bad_wire)
        .expect_err("a payload missing nolus_receiver must fail deserialization");
}

// The typed→wire `From` threads `nolus_receiver` unchanged: building a typed
// `OpenProfitParams`, converting to the wire shape, and serialising yields the
// same receiver the typed value carried.
#[test]
fn typed_to_wire_threads_nolus_receiver() {
    let typed = OpenProfitParams::new(7, sample_nolus_receiver());
    let wire = remote_profit_wire::msg::OpenProfitParams::from(&typed);
    assert_eq!(SAMPLE_NOLUS_RECEIVER, wire.nolus_receiver().as_str());
    assert_eq!(
        typed.expected_instance_ordinal(),
        wire.expected_instance_ordinal(),
    );
}

// ---------------------------------------------------------------------------
// 6. SwapParams invariant: coin_in and min_out currencies differ, both > 0
// ---------------------------------------------------------------------------

#[test]
fn swap_params_distinct_currencies_ok() {
    let params = SwapParams::new(
        Coin::<PaymentC1>::new(1000).into(),
        Coin::<PaymentC2>::new(42).into(),
    )
    .expect("distinct non-zero amounts must be accepted");
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
fn swap_params_zero_coin_in_rejected() {
    let res = SwapParams::new(
        Coin::<PaymentC1>::new(0).into(),
        Coin::<PaymentC2>::new(42).into(),
    );
    assert!(matches!(res, Err(Error::ZeroSwapAmount)));
}

#[test]
fn swap_params_zero_min_out_rejected() {
    let res = SwapParams::new(
        Coin::<PaymentC1>::new(1000).into(),
        Coin::<PaymentC2>::new(0).into(),
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
    let params = TransferOutParams::new(Coin::<PaymentC3>::new(1000).into())
        .expect("non-zero amount must be accepted");
    assert!(params.invariant_held());
}

#[test]
fn transfer_out_params_zero_rejected() {
    let res = TransferOutParams::new(Coin::<PaymentC3>::new(0).into());
    assert!(matches!(res, Err(Error::ZeroTransferAmount)));
}

#[test]
fn transfer_out_params_deserialize_zero_rejected() {
    let bad_wire = r#"{"amount":{"amount":"0","ticker":"LC1"}}"#;
    serde_json::from_str::<TransferOutParams>(bad_wire)
        .expect_err("zero amount must fail deserialization");
}

// ---------------------------------------------------------------------------
// 8. Wire-protocol constants
// ---------------------------------------------------------------------------

#[test]
fn version_constant_pinned() {
    assert_eq!("nls-remote-profit.v1", VERSION);
}

#[test]
fn port_prefix_constant_pinned() {
    assert_eq!("nls-remote-profit.", PORT_PREFIX);
}

#[test]
fn port_id_for_dex_concatenates_prefix() {
    assert_eq!("nls-remote-profit.astroport", port_id_for("astroport"));
}

#[test]
fn protocol_version_round_trip_pinned() {
    assert_round_trip_eq(r#""nls-remote-profit.v1""#, &ProtocolVersion);
}

// ---------------------------------------------------------------------------
// 9. Cross-crate typed↔wire envelope byte-equality (drift tripwire)
//
// The typed `PacketEnvelope` (decoded in production) and the wire
// `PacketEnvelope` (consumed by the Solana counterpart) each declare their own
// field list and serde attributes. For every `Operation` variant, the
// semantically-equivalent typed and wire envelopes must serialize
// byte-identically, and the wire bytes must decode into the typed envelope.
// Any drift in either side's field order, field name, or serde attribute
// breaks these.
// ---------------------------------------------------------------------------

#[test]
fn envelope_open_profit_typed_matches_wire() {
    assert_typed_wire_envelopes_match(Operation::OpenProfit(sample_open_profit_params()));
}

#[test]
fn envelope_swap_typed_matches_wire() {
    assert_typed_wire_envelopes_match(Operation::Swap(sample_swap_params()));
}

#[test]
fn envelope_transfer_out_typed_matches_wire() {
    assert_typed_wire_envelopes_match(Operation::TransferOut(sample_transfer_out_params()));
}

#[test]
fn envelope_close_profit_typed_matches_wire() {
    assert_typed_wire_envelopes_match(Operation::CloseProfit(CloseProfitParams {}));
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

fn assert_typed_wire_envelopes_match(operation: Operation) {
    const NONCE: u64 = 7;

    let wire = remote_profit_wire::envelope::PacketEnvelope {
        operation: remote_profit_wire::msg::Operation::from(&operation),
        version: ProtocolVersion,
        nonce: NONCE,
    };
    let typed = PacketEnvelope {
        operation,
        version: ProtocolVersion,
        nonce: NONCE,
    };

    let wire_json = serde_json::to_string(&wire).expect("the wire envelope must serialize");
    assert_eq!(
        wire_json,
        serde_json::to_string(&typed).expect("the typed envelope must serialize"),
    );

    let decoded: PacketEnvelope =
        serde_json::from_str(&wire_json).expect("wire bytes must decode into the typed envelope");
    assert_eq!(typed, decoded);
}

/// A real bech32 Nolus account address (32-byte witness), valid checksum.
const SAMPLE_NOLUS_RECEIVER: &str =
    "nolus1mf6ptkssddfmxvhdx0ech0k03ktp6kf9yk59renau2gvht3nq2gqkxgywu";

fn sample_open_profit_params() -> OpenProfitParams {
    OpenProfitParams::new(7, sample_nolus_receiver())
}

fn sample_nolus_receiver() -> NolusReceiver {
    NolusReceiver::new(SAMPLE_NOLUS_RECEIVER).expect("sample address is a valid bech32 Nolus addr")
}

fn sample_swap_params() -> SwapParams {
    SwapParams::new(
        Coin::<PaymentC1>::new(1000).into(),
        Coin::<PaymentC2>::new(42).into(),
    )
    .expect("sample uses two distinct non-zero amounts")
}

fn sample_transfer_out_params() -> TransferOutParams {
    TransferOutParams::new(Coin::<PaymentC3>::new(1000).into())
        .expect("sample uses a non-zero amount")
}
