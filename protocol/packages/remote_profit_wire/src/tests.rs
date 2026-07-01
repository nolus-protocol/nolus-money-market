//! Wire-format byte-pin tests for the cross-chain `remote_profit` protocol.
//!
//! Every literal-JSON pin below locks the wire encoding so the Solana side can
//! rely on a stable surface — **any edit to a literal pin is a breaking
//! protocol change and MUST bump [`crate::VERSION`]**, with one exception: an
//! additive field marked `#[serde(default)]` (e.g. `nonce`) extends the wire
//! without a version bump, because updated consumers decode both the old and
//! new shapes and the rollout is coordinated consumer-first rather than
//! signalled by the version.

use std::fmt::Debug;

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::{
    PORT_PREFIX, VERSION,
    callback::{
        OPERATION_ERR_MAX_BYTES, RemoteErrorMessage, RemoteOperationOutcome, RemoteProfitCallback,
    },
    coin::WireCoin,
    envelope::PacketEnvelope,
    error::Error,
    msg::{CloseProfitParams, OpenProfitParams, Operation, SwapParams, TransferOutParams},
    nolus_receiver::{NOLUS_RECEIVER_MAX_BYTES, NolusReceiver},
    port_id_for,
    profit_id::{REMOTE_PROFIT_ID_MAX_BYTES, RemoteProfitId},
    response::{
        CloseProfitResponse, OpenProfitResponse, OperationResponse, SwapResponse,
        TransferOutResponse,
    },
    ticker::Ticker,
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

#[test]
fn close_profit_msg_serde() {
    let value = Operation::CloseProfit(CloseProfitParams {});
    assert_round_trip_eq(r#"{"close_profit":{}}"#, &value);
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

#[test]
fn close_profit_response_serde() {
    let value = OperationResponse::CloseProfit(CloseProfitResponse {});
    assert_round_trip_eq(r#"{"close_profit":{}}"#, &value);
}

// ---------------------------------------------------------------------------
// 3. RemoteProfitCallback variants — round-trip + literal JSON
// ---------------------------------------------------------------------------

#[test]
fn callback_operation_ok_serde() {
    let value =
        RemoteOperationOutcome::OperationOk(OperationResponse::CloseProfit(CloseProfitResponse {}));
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

// The callback wraps a per-emission `nonce` alongside the outcome, so the
// controller can correlate an acknowledgment to the exact packet that
// solicited it; the nonce is the FIRST field and round-trips on the wire.
#[test]
fn callback_round_trips_with_nonce() {
    let value = RemoteProfitCallback {
        nonce: 7,
        outcome: RemoteOperationOutcome::OperationOk(OperationResponse::CloseProfit(
            CloseProfitResponse {},
        )),
    };
    assert_round_trip_eq(
        r#"{"nonce":7,"outcome":{"operation_ok":{"close_profit":{}}}}"#,
        &value,
    );
}

// ---------------------------------------------------------------------------
// 4. PacketEnvelope — round-trip + literal JSON (Option A: no identity field)
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

// The envelope carries a per-emission `nonce` as its last field (after
// `version`); a non-zero nonce round-trips byte-identically so the Solana side
// can echo it back unchanged on the acknowledgment.
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

// `nonce` is `#[serde(default)]`, so a payload that predates the field — a
// legacy packet from a counterparty that has not yet upgraded — decodes with
// `nonce == 0` rather than being rejected.
#[test]
fn packet_envelope_decodes_without_nonce_to_zero() {
    let wire = r#"{"operation":{"close_profit":{}},"version":"nls-remote-profit.v1"}"#;
    let envelope: PacketEnvelope =
        serde_json::from_str(wire).expect("a payload without nonce must default it to zero");
    assert_eq!(0, envelope.nonce);
}

// ---------------------------------------------------------------------------
// 5. OpenProfitParams — minimal singleton-establishment payload
// ---------------------------------------------------------------------------

#[test]
fn open_profit_params_round_trips_with_nolus_receiver() {
    let value = OpenProfitParams::new(7, sample_nolus_receiver());
    assert_round_trip_eq(
        r#"{"expected_instance_ordinal":7,"nolus_receiver":"nolus1mf6ptkssddfmxvhdx0ech0k03ktp6kf9yk59renau2gvht3nq2gqkxgywu"}"#,
        &value,
    );
    assert_eq!(7, value.expected_instance_ordinal());
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

#[test]
fn open_profit_params_rejects_unknown_field() {
    let bad_wire = r#"{"expected_instance_ordinal":7,"nolus_receiver":"nolus1mf6ptkssddfmxvhdx0ech0k03ktp6kf9yk59renau2gvht3nq2gqkxgywu","lpn_currency":"LPN"}"#;
    serde_json::from_str::<OpenProfitParams>(bad_wire)
        .expect_err("an unknown field must fail deserialization");
}

// `nolus_receiver` is a plain required field (no `#[serde(default)]`): the
// remote_profit crates are undeployed, so the old single-field shape must be
// rejected outright rather than decoded with a defaulted receiver.
#[test]
fn open_profit_params_rejects_missing_nolus_receiver() {
    let bad_wire = r#"{"expected_instance_ordinal":7}"#;
    serde_json::from_str::<OpenProfitParams>(bad_wire)
        .expect_err("a payload missing nolus_receiver must fail deserialization");
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
// 6b. RemoteProfitId — round-trip + validation
// ---------------------------------------------------------------------------

#[test]
fn remote_profit_id_round_trip_is_bare_string() {
    let id = RemoteProfitId::new("So1RayProfit").expect("base58 profit id");
    assert_round_trip_eq(r#""So1RayProfit""#, &id);
}

#[test]
fn remote_profit_id_accessors_expose_the_payload() {
    let id = RemoteProfitId::new("So1RayProfit").expect("base58 profit id");
    assert_eq!("So1RayProfit", id.as_str());
    assert_eq!("So1RayProfit", AsRef::<str>::as_ref(&id));
    assert_eq!("So1RayProfit", id.to_string());
}

#[test]
fn remote_profit_id_empty_rejected() {
    let res = RemoteProfitId::new("");
    assert!(matches!(res, Err(Error::RemoteProfitIdEmpty)));
}

#[test]
fn remote_profit_id_at_cap_accepted() {
    let payload = "a".repeat(REMOTE_PROFIT_ID_MAX_BYTES);
    RemoteProfitId::new(payload).expect("payload at the cap must be accepted");
}

#[test]
fn remote_profit_id_over_cap_rejected() {
    let payload = "a".repeat(REMOTE_PROFIT_ID_MAX_BYTES + 1);
    let res = RemoteProfitId::new(payload);
    assert!(matches!(
        res,
        Err(Error::RemoteProfitIdTooLong {
            actual,
            max: REMOTE_PROFIT_ID_MAX_BYTES,
        }) if actual == REMOTE_PROFIT_ID_MAX_BYTES + 1,
    ));
}

#[test]
fn remote_profit_id_non_base58_rejected() {
    // The base58 alphabet excludes 0, O, I, l.
    for &bad in &[
        "0badId",
        "OBadId",
        "IbadId",
        "lbadId",
        "with-hyphen",
        "with space",
    ] {
        let res = RemoteProfitId::new(bad);
        assert!(
            matches!(res, Err(Error::RemoteProfitIdInvalidCharacter { .. })),
            "expected rejection for {bad:?}, got {res:?}",
        );
    }
}

#[test]
fn remote_profit_id_deserialize_empty_rejected() {
    serde_json::from_str::<RemoteProfitId>(r#""""#)
        .expect_err("empty profit id must fail deserialization");
}

#[test]
fn remote_profit_id_deserialize_non_base58_rejected() {
    serde_json::from_str::<RemoteProfitId>(r#""bad-id""#)
        .expect_err("non-base58 character must fail deserialization");
}

#[test]
fn remote_profit_id_deserialize_over_cap_rejected() {
    let payload = "a".repeat(REMOTE_PROFIT_ID_MAX_BYTES + 1);
    let bad_wire = format!(r#""{payload}""#);
    serde_json::from_str::<RemoteProfitId>(&bad_wire)
        .expect_err("over-cap profit id must fail deserialization");
}

// ---------------------------------------------------------------------------
// 6c. NolusReceiver — bech32 round-trip + validation
// ---------------------------------------------------------------------------

#[test]
fn nolus_receiver_accepts_a_valid_bech32() {
    let receiver =
        NolusReceiver::new(SAMPLE_NOLUS_RECEIVER).expect("a valid bech32 Nolus addr is accepted");
    assert_eq!(SAMPLE_NOLUS_RECEIVER, receiver.as_str());
    assert_round_trip_eq(
        r#""nolus1mf6ptkssddfmxvhdx0ech0k03ktp6kf9yk59renau2gvht3nq2gqkxgywu""#,
        &receiver,
    );
}

#[test]
fn nolus_receiver_rejects_a_malformed_address() {
    // empty → wrong HRP → corrupted checksum → non-bech32 char → mixed case:
    // each is a malformed address that must be rejected before construction.
    let cases = [
        "",
        "cosmos1mf6ptkssddfmxvhdx0ech0k03ktp6kf9yk59renau2gvht3nq2gqkxgywu",
        "nolus1mf6ptkssddfmxvhdx0ech0k03ktp6kf9yk59renau2gvht3nq2gqkxgywv",
        "nolus1bio",
        "NOLUS1MF6PTKSSDDFMXVHDX0ECH0K03KTP6KF9YK59RENAU2GVHT3NQ2GQKXGYWU",
    ];
    for bad in cases {
        let res = NolusReceiver::new(bad);
        assert!(
            matches!(
                res,
                Err(Error::NolusReceiverEmpty
                    | Error::NolusReceiverWrongHrp
                    | Error::NolusReceiverInvalidBech32)
            ),
            "expected typed rejection for {bad:?}, got {res:?}",
        );
    }

    let over_cap = format!("nolus1{}", "q".repeat(NOLUS_RECEIVER_MAX_BYTES));
    assert!(matches!(
        NolusReceiver::new(over_cap.clone()),
        Err(Error::NolusReceiverTooLong {
            actual,
            max: NOLUS_RECEIVER_MAX_BYTES,
        }) if actual == over_cap.len(),
    ));
}

#[test]
fn nolus_receiver_deserialize_malformed_rejected() {
    serde_json::from_str::<NolusReceiver>(r#""cosmos1qqqqqqqq""#)
        .expect_err("a wrong-HRP address must fail deserialization");
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
    serde_json::from_str::<WireCoin>(bad_wire).expect_err("empty amount must fail deserialization");
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

#[test]
fn wire_coin_large_amount_canonical_round_trip() {
    let value = WireCoin::new(u128::MAX, Ticker::new("NLS"));
    let expected = format!(r#"{{"amount":"{}","ticker":"NLS"}}"#, u128::MAX);
    assert_round_trip_eq(&expected, &value);
}

// ---------------------------------------------------------------------------
// 9. ProtocolVersion cross-protocol isolation guard
// ---------------------------------------------------------------------------

#[test]
fn protocol_version_round_trip_pinned() {
    assert_round_trip_eq(r#""nls-remote-profit.v1""#, &ProtocolVersion);
}

#[test]
fn protocol_version_rejects_sibling_protocol() {
    serde_json::from_str::<ProtocolVersion>(r#""nls-remote-lease.v1""#)
        .expect_err("the sibling remote-lease version must be rejected");
}

#[test]
fn protocol_version_rejects_arbitrary_string() {
    serde_json::from_str::<ProtocolVersion>(r#""whatever.v9""#)
        .expect_err("an arbitrary version string must be rejected");
}

// ---------------------------------------------------------------------------
// 10. Wire-protocol constants
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
        WireCoin::new(1000, Ticker::new("NLS")),
        WireCoin::new(42, Ticker::new("LPN")),
    )
    .expect("sample uses two distinct non-zero amounts")
}

fn sample_transfer_out_params() -> TransferOutParams {
    TransferOutParams::new(WireCoin::new(1000, Ticker::new("LC1")))
        .expect("sample uses a non-zero amount")
}
