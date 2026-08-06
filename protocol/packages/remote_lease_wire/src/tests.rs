//! Wire-format byte-pin tests for the cross-chain `remote_lease` protocol.
//!
//! Acceptance criterion (GH #626): every literal-JSON pin below must equal
//! the JSON the typed `remote_lease` crate emits for the same logical value.
//! The cross-surface integration test under `remote_lease/tests/` validates
//! that equivalence end-to-end; this module locks the wire encoding so the
//! Solana side can rely on a stable surface.
//!
//! **Editing a pin.** For the pins that cross the IBC channel — §1 `Operation`,
//! §2 `OperationResponse`, §4 `PacketEnvelope` — an edit is a breaking protocol
//! change. While [`crate::VERSION`] is unreleased those land **in place**: the
//! breaking-change signal is a minor bump of this crate's version plus the
//! paired `rev` bump on the counterparty side, both reviewable diffs.
//! [`crate::VERSION`] itself is bumped only once the protocol is live and two
//! generations must coexist, because [`crate::version::ProtocolVersion`]
//! rejects a mismatch at the deserialiser and a bump therefore breaks every
//! in-flight packet. §3 `RemoteLeaseCallback` is the controller-to-lease
//! `ExecuteMsg` payload and never crosses the channel at all. §10 is the
//! handshake version grammar: it too is a paired cross-repo change, and it is
//! the one pin the counterparty *parses* rather than deserialises, so its
//! literals are what keep the two renderings byte-identical.

use std::fmt::Debug;

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::{
    PORT_PREFIX, VERSION,
    callback::{
        OPERATION_ERR_MAX_BYTES, REMOTE_ERROR_CODE_MAX_BYTES, RemoteError, RemoteErrorKind,
        RemoteErrorMessage, RemoteLeaseCallback,
    },
    channel_version::{
        CHANNEL_VERSION_MAX_BYTES, ICS20_CHANNEL_ID_MAX_BYTES, Ics20ChannelId,
        bounded_channel_version,
    },
    coin::WireCoin,
    envelope::{LeaseAddrOnWire, PacketEnvelope},
    error::Error,
    lease_id::{REMOTE_LEASE_ID_MAX_BYTES, RemoteLeaseId},
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
        r#"{"swap":{"one":{"coin_in":{"amount":"1000","ticker":"NLS"},"min_out":{"amount":"42","ticker":"LPN"}}}}"#,
        &value,
    );
}

#[test]
fn swap_msg_two_serde() {
    let value = Operation::Swap(
        SwapParams::two(
            WireCoin::new(1000, Ticker::new("NLS")),
            WireCoin::new(500, Ticker::new("LC1")),
            WireCoin::new(42, Ticker::new("LPN")),
        )
        .expect("sample uses three distinct non-zero amounts"),
    );
    assert_round_trip_eq(
        r#"{"swap":{"two":{"coin_in_1":{"amount":"1000","ticker":"NLS"},"coin_in_2":{"amount":"500","ticker":"LC1"},"min_out":{"amount":"42","ticker":"LPN"}}}}"#,
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
        remote_lease_id: RemoteLeaseId::new("So1RayLease1").expect("base58 lease id"),
    });
    assert_round_trip_eq(
        r#"{"open_lease":{"remote_lease_id":"So1RayLease1"}}"#,
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
fn callback_operation_err_min_out_unmet_serde() {
    assert_round_trip_eq(
        r#"{"operation_err":{"kind":"min_out_unmet","message":"ibc-solray: credit below min"}}"#,
        &operation_err(RemoteErrorKind::MinOutUnmet, "ibc-solray: credit below min"),
    );
}

#[test]
fn callback_operation_err_permanent_serde() {
    assert_round_trip_eq(
        r#"{"operation_err":{"kind":"permanent","message":"dex pool drained"}}"#,
        &operation_err(RemoteErrorKind::Permanent, "dex pool drained"),
    );
}

#[test]
fn callback_operation_err_transient_serde() {
    assert_round_trip_eq(
        r#"{"operation_err":{"kind":"transient","message":"host clock unavailable"}}"#,
        &operation_err(RemoteErrorKind::Transient, "host clock unavailable"),
    );
}

#[test]
fn callback_operation_timeout_serde() {
    let value = RemoteLeaseCallback::OperationTimeout;
    assert_round_trip_eq(r#""operation_timeout""#, &value);
}

// ---------------------------------------------------------------------------
// 3a. RemoteErrorKind — the published token vocabulary
// ---------------------------------------------------------------------------

// The counterparty frames acknowledgements with `as_wire`; this crate decodes
// the same tokens through serde. Pinning both against one literal is what keeps
// the two representations from drifting apart.
#[test]
fn error_kind_tokens_pin_the_published_contract() {
    fn assert_token(expected: &str, kind: RemoteErrorKind) {
        assert_eq!(expected, kind.as_wire());
        assert_eq!(
            format!(r#""{expected}""#),
            serde_json::to_string(&kind).expect("a unit variant must serialize"),
        );
    }

    assert_token("min_out_unmet", RemoteErrorKind::MinOutUnmet);
    assert_token("permanent", RemoteErrorKind::Permanent);
    assert_token("transient", RemoteErrorKind::Transient);
}

#[test]
fn error_kind_tokens_are_frame_safe() {
    for kind in [
        RemoteErrorKind::MinOutUnmet,
        RemoteErrorKind::Permanent,
        RemoteErrorKind::Transient,
    ] {
        let token = kind.as_wire();
        assert!(!token.is_empty(), "{token} must not be empty");
        assert!(
            token.len() <= REMOTE_ERROR_CODE_MAX_BYTES,
            "{token} must fit the code cap",
        );
        assert!(
            token
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_'),
            "{token} must stay within the frame charset",
        );
    }
}

// An unknown kind must FAIL, not degrade to a default. A default would invent a
// meaning the counterparty never sent and then route funds with it; the two
// ends deploy in lockstep, so an unknown token is a deployment fault.
#[test]
fn callback_operation_err_unknown_kind_rejected() {
    serde_json::from_str::<RemoteLeaseCallback>(
        r#"{"operation_err":{"kind":"future_class","message":"x"}}"#,
    )
    .expect_err("an unrecognised kind must not decode");
}

#[test]
fn callback_operation_err_missing_kind_rejected() {
    serde_json::from_str::<RemoteLeaseCallback>(r#"{"operation_err":{"message":"x"}}"#)
        .expect_err("a producer omitting the kind is not speaking this wire generation");
}

#[test]
fn callback_operation_err_unknown_field_rejected() {
    serde_json::from_str::<RemoteLeaseCallback>(
        r#"{"operation_err":{"kind":"permanent","message":"x","extra":1}}"#,
    )
    .expect_err("`deny_unknown_fields` must reach the inner object, not just the enum");
}

// ---------------------------------------------------------------------------
// 3b. The acknowledgement code frame — `format_ack` / `parse_ack`
// ---------------------------------------------------------------------------

#[test]
fn ack_frame_round_trips_every_kind() {
    for kind in [
        RemoteErrorKind::MinOutUnmet,
        RemoteErrorKind::Permanent,
        RemoteErrorKind::Transient,
    ] {
        let parsed = RemoteError::parse_ack(RemoteError::format_ack(kind, "ibc-solray: boom"))
            .expect("a self-rendered frame must parse back");

        assert_eq!(kind, parsed.kind());
        assert_eq!("ibc-solray: boom", parsed.message().as_str());
    }
}

#[test]
fn ack_frame_strips_the_code_from_the_retained_prose() {
    let parsed = RemoteError::parse_ack("[min_out_unmet] ibc-solray: credit below min")
        .expect("a well-formed frame must parse");

    assert_eq!(RemoteErrorKind::MinOutUnmet, parsed.kind());
    assert_eq!("ibc-solray: credit below min", parsed.message().as_str());
}

#[test]
fn ack_frame_without_trailing_prose_parses_to_an_empty_message() {
    let parsed = RemoteError::parse_ack("[permanent]").expect("a bare frame must parse");

    assert_eq!(RemoteErrorKind::Permanent, parsed.kind());
    assert_eq!("", parsed.message().as_str());
}

#[test]
fn ack_frame_malformed_rejected() {
    // Every shape that is not exactly `[<known token>]`: absent, unclosed, no
    // opening sigil, empty, wrong case, over-long, and a multi-byte char in the
    // token position — the last one guards the byte-indexed scan against a
    // char-boundary panic.
    for ack in [
        "dex pool drained",
        "[min_out_unmet ibc-solray: x",
        "min_out_unmet] x",
        "[] x",
        "[MIN_OUT_UNMET] x",
        "[min out unmet] x",
        "[aaaaaaaaaaaaaaaaa] x",
        "[Мin_out_unmet] x",
    ] {
        assert!(
            matches!(
                RemoteError::parse_ack(ack),
                Err(Error::CallbackErrorCodeMissing {
                    max: REMOTE_ERROR_CODE_MAX_BYTES
                }),
            ),
            "{ack:?} must be rejected as a malformed frame",
        );
    }
}

#[test]
fn ack_frame_unknown_code_rejected() {
    assert!(matches!(
        RemoteError::parse_ack("[future_class] ibc-solray: x"),
        Err(Error::CallbackErrorCodeUnknown { code }) if code == "future_class",
    ));
}

// The counterparty truncates the TAIL to fit the cap, so a head-position code
// survives structurally. This pins that property rather than leaving it to luck.
#[test]
fn ack_frame_code_survives_a_message_at_the_cap() {
    let framed = RemoteError::format_ack(RemoteErrorKind::MinOutUnmet, "");
    let ack = format!(
        "{framed}{}",
        "x".repeat(OPERATION_ERR_MAX_BYTES - framed.len())
    );
    assert_eq!(OPERATION_ERR_MAX_BYTES, ack.len());

    assert_eq!(
        RemoteErrorKind::MinOutUnmet,
        RemoteError::parse_ack(ack)
            .expect("an acknowledgement at the cap must parse")
            .kind(),
    );
}

// The cap applies to the prose AFTER the frame is stripped, so an
// acknowledgement over-long only by its frame is now accepted.
#[test]
fn ack_frame_prose_at_cap_accepted_and_over_cap_rejected() {
    let at_cap = RemoteError::format_ack(
        RemoteErrorKind::Transient,
        &"x".repeat(OPERATION_ERR_MAX_BYTES),
    );
    assert!(at_cap.len() > OPERATION_ERR_MAX_BYTES);
    assert_eq!(
        OPERATION_ERR_MAX_BYTES,
        RemoteError::parse_ack(at_cap)
            .expect("prose at the cap must be accepted once the frame is stripped")
            .message()
            .as_str()
            .len(),
    );

    let over_cap = RemoteError::format_ack(
        RemoteErrorKind::Transient,
        &"x".repeat(OPERATION_ERR_MAX_BYTES + 1),
    );
    assert!(matches!(
        RemoteError::parse_ack(over_cap),
        Err(Error::CallbackErrorTooLong {
            actual,
            max: OPERATION_ERR_MAX_BYTES,
        }) if actual == OPERATION_ERR_MAX_BYTES + 1,
    ));
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

#[test]
fn callback_error_message_from_static_accepted() {
    let value = RemoteErrorMessage::from_static("timeout");
    assert_eq!("timeout", value.as_str());
    assert_round_trip_eq(
        r#"{"operation_err":{"kind":"transient","message":"timeout"}}"#,
        &RemoteLeaseCallback::OperationErr(RemoteError::new(RemoteErrorKind::Transient, value)),
    );
}

// `from_static` only `debug_assert!`s its length contract, so the panic is
// observable solely in debug builds — the test is gated to match.
#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "OPERATION_ERR_MAX_BYTES")]
fn callback_error_message_from_static_over_cap_panics_in_debug() {
    let over_cap: &'static str =
        Box::leak("x".repeat(OPERATION_ERR_MAX_BYTES + 1).into_boxed_str());
    let _ = RemoteErrorMessage::from_static(over_cap);
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

// The rejected version is counterparty bytes and the serde error text reaches
// logs and events, so it must not carry an unbounded echo.
#[test]
fn packet_envelope_version_mismatch_error_is_bounded() {
    let oversized = "v".repeat(CHANNEL_VERSION_MAX_BYTES * 8);
    let bad_wire = format!(
        r#"{{"lease":"nolus1leaseaddr","operation":{{"close_lease":{{}}}},"version":"{oversized}"}}"#
    );
    let message = serde_json::from_str::<PacketEnvelope>(&bad_wire)
        .expect_err("mismatched protocol version must fail deserialization")
        .to_string();

    assert!(
        message.contains(&oversized[..CHANNEL_VERSION_MAX_BYTES]),
        "the capped echo must be retained",
    );
    assert!(
        !message.contains(&oversized[..CHANNEL_VERSION_MAX_BYTES + 1]),
        "not one byte past the cap may be retained",
    );
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
// 5. OpenLeaseParams invariant: lpn != asset (downpayment may overlap either)
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
fn open_lease_params_downpayment_equals_lpn_accepted() {
    OpenLeaseParams::new(
        7,
        Ticker::new("NLS"),
        Ticker::new("NLS"),
        Ticker::new("LC1"),
    )
    .expect("downpayment in lpn currency must be accepted");
}

#[test]
fn open_lease_params_downpayment_equals_asset_accepted() {
    OpenLeaseParams::new(
        7,
        Ticker::new("NLS"),
        Ticker::new("LPN"),
        Ticker::new("NLS"),
    )
    .expect("downpayment in asset currency must be accepted");
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
    let bad_wire = r#"{"expected_instance_ordinal":7,"downpayment_currency":"NLS","lpn_currency":"LPN","asset_currency":"LPN"}"#;
    serde_json::from_str::<OpenLeaseParams>(bad_wire)
        .expect_err("lpn==asset must fail deserialization");
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
fn swap_params_one_distinct_currencies_ok() {
    let params = SwapParams::one(
        WireCoin::new(1000, Ticker::new("NLS")),
        WireCoin::new(42, Ticker::new("LPN")),
    )
    .expect("distinct non-zero amounts must be accepted");
    assert!(params.invariant_held());
}

#[test]
fn swap_params_one_same_currency_rejected() {
    let res = SwapParams::one(
        WireCoin::new(1000, Ticker::new("NLS")),
        WireCoin::new(42, Ticker::new("NLS")),
    );
    assert!(matches!(res, Err(Error::SameSwapCurrency)));
}

#[test]
fn swap_params_one_zero_coin_in_rejected() {
    let res = SwapParams::one(
        WireCoin::new(0, Ticker::new("NLS")),
        WireCoin::new(42, Ticker::new("LPN")),
    );
    assert!(matches!(res, Err(Error::ZeroSwapAmount)));
}

#[test]
fn swap_params_one_zero_min_out_rejected() {
    let res = SwapParams::one(
        WireCoin::new(1000, Ticker::new("NLS")),
        WireCoin::new(0, Ticker::new("LPN")),
    );
    assert!(matches!(res, Err(Error::ZeroSwapAmount)));
}

#[test]
fn swap_params_one_deserialize_invariant_violation_rejected() {
    let bad_wire = r#"{"one":{"coin_in":{"amount":"1000","ticker":"NLS"},"min_out":{"amount":"42","ticker":"NLS"}}}"#;
    serde_json::from_str::<SwapParams>(bad_wire)
        .expect_err("invariant violation must fail deserialization");
}

#[test]
fn swap_params_one_deserialize_zero_amount_rejected() {
    let bad_wire = r#"{"one":{"coin_in":{"amount":"0","ticker":"NLS"},"min_out":{"amount":"42","ticker":"LPN"}}}"#;
    serde_json::from_str::<SwapParams>(bad_wire)
        .expect_err("zero coin_in must fail deserialization");
}

#[test]
fn swap_params_two_distinct_currencies_ok() {
    let params = SwapParams::two(
        WireCoin::new(1000, Ticker::new("NLS")),
        WireCoin::new(500, Ticker::new("LC1")),
        WireCoin::new(42, Ticker::new("LPN")),
    )
    .expect("three distinct non-zero amounts must be accepted");
    assert!(params.invariant_held());
}

#[test]
fn swap_params_two_zero_coin_in_1_rejected() {
    let res = SwapParams::two(
        WireCoin::new(0, Ticker::new("NLS")),
        WireCoin::new(500, Ticker::new("LC1")),
        WireCoin::new(42, Ticker::new("LPN")),
    );
    assert!(matches!(res, Err(Error::ZeroSwapAmount)));
}

#[test]
fn swap_params_two_zero_coin_in_2_rejected() {
    let res = SwapParams::two(
        WireCoin::new(1000, Ticker::new("NLS")),
        WireCoin::new(0, Ticker::new("LC1")),
        WireCoin::new(42, Ticker::new("LPN")),
    );
    assert!(matches!(res, Err(Error::ZeroSwapAmount)));
}

#[test]
fn swap_params_two_zero_min_out_rejected() {
    let res = SwapParams::two(
        WireCoin::new(1000, Ticker::new("NLS")),
        WireCoin::new(500, Ticker::new("LC1")),
        WireCoin::new(0, Ticker::new("LPN")),
    );
    assert!(matches!(res, Err(Error::ZeroSwapAmount)));
}

#[test]
fn swap_params_two_coin_in_1_same_as_min_out_rejected() {
    let res = SwapParams::two(
        WireCoin::new(1000, Ticker::new("LPN")),
        WireCoin::new(500, Ticker::new("LC1")),
        WireCoin::new(42, Ticker::new("LPN")),
    );
    assert!(matches!(res, Err(Error::SameSwapCurrency)));
}

#[test]
fn swap_params_two_coin_in_2_same_as_min_out_rejected() {
    let res = SwapParams::two(
        WireCoin::new(1000, Ticker::new("NLS")),
        WireCoin::new(500, Ticker::new("LPN")),
        WireCoin::new(42, Ticker::new("LPN")),
    );
    assert!(matches!(res, Err(Error::SameSwapCurrency)));
}

#[test]
fn swap_params_two_coin_in_1_same_as_coin_in_2_rejected() {
    let res = SwapParams::two(
        WireCoin::new(1000, Ticker::new("NLS")),
        WireCoin::new(500, Ticker::new("NLS")),
        WireCoin::new(42, Ticker::new("LPN")),
    );
    assert!(matches!(res, Err(Error::DuplicateSwapInputCurrency)));
}

#[test]
fn swap_params_two_deserialize_invariant_violation_rejected() {
    let bad_wire = r#"{"two":{"coin_in_1":{"amount":"1000","ticker":"NLS"},"coin_in_2":{"amount":"500","ticker":"NLS"},"min_out":{"amount":"42","ticker":"LPN"}}}"#;
    serde_json::from_str::<SwapParams>(bad_wire)
        .expect_err("duplicate input currencies must fail deserialization");
}

// ---------------------------------------------------------------------------
// 6b. RemoteLeaseId — round-trip + validation
// ---------------------------------------------------------------------------

#[test]
fn remote_lease_id_round_trip_is_bare_string() {
    let id = RemoteLeaseId::new("So1RayLease1").expect("base58 lease id");
    assert_round_trip_eq(r#""So1RayLease1""#, &id);
}

#[test]
fn remote_lease_id_accessors_expose_the_payload() {
    let id = RemoteLeaseId::new("So1RayLease1").expect("base58 lease id");
    assert_eq!("So1RayLease1", id.as_str());
    assert_eq!("So1RayLease1", AsRef::<str>::as_ref(&id));
    assert_eq!("So1RayLease1", id.to_string());
}

#[test]
fn remote_lease_id_empty_rejected() {
    let res = RemoteLeaseId::new("");
    assert!(matches!(res, Err(Error::RemoteLeaseIdEmpty)));
}

#[test]
fn remote_lease_id_at_cap_accepted() {
    let payload = "a".repeat(REMOTE_LEASE_ID_MAX_BYTES);
    RemoteLeaseId::new(payload).expect("payload at the cap must be accepted");
}

#[test]
fn remote_lease_id_over_cap_rejected() {
    let payload = "a".repeat(REMOTE_LEASE_ID_MAX_BYTES + 1);
    let res = RemoteLeaseId::new(payload);
    assert!(matches!(
        res,
        Err(Error::RemoteLeaseIdTooLong {
            actual,
            max: REMOTE_LEASE_ID_MAX_BYTES,
        }) if actual == REMOTE_LEASE_ID_MAX_BYTES + 1,
    ));
}

#[test]
fn remote_lease_id_non_base58_rejected() {
    // The base58 alphabet excludes 0, O, I, l.
    for &bad in &[
        "0badId",
        "OBadId",
        "IbadId",
        "lbadId",
        "with-hyphen",
        "with space",
    ] {
        let res = RemoteLeaseId::new(bad);
        assert!(
            matches!(res, Err(Error::RemoteLeaseIdInvalidCharacter { .. })),
            "expected rejection for {bad:?}, got {res:?}",
        );
    }
}

#[test]
fn remote_lease_id_deserialize_empty_rejected() {
    serde_json::from_str::<RemoteLeaseId>(r#""""#)
        .expect_err("empty lease id must fail deserialization");
}

#[test]
fn remote_lease_id_deserialize_non_base58_rejected() {
    serde_json::from_str::<RemoteLeaseId>(r#""bad-id""#)
        .expect_err("non-base58 character must fail deserialization");
}

#[test]
fn remote_lease_id_deserialize_over_cap_rejected() {
    let payload = "a".repeat(REMOTE_LEASE_ID_MAX_BYTES + 1);
    let bad_wire = format!(r#""{payload}""#);
    serde_json::from_str::<RemoteLeaseId>(&bad_wire)
        .expect_err("over-cap lease id must fail deserialization");
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
// 10. Ics20ChannelId and the `+transfer=` suffixed handshake version
// ---------------------------------------------------------------------------

#[test]
fn ics20_channel_id_caps_pinned() {
    assert_eq!(13, ICS20_CHANNEL_ID_MAX_BYTES);
    assert_eq!(42, CHANNEL_VERSION_MAX_BYTES);
}

#[test]
fn ics20_channel_id_render_round_trips_the_ordinal_bounds() {
    for ics20_channel in ["channel-0", "channel-1", "channel-65535"] {
        assert_eq!(ics20_channel, channel_id(ics20_channel).to_string());
    }
}

#[test]
fn channel_version_composition_pinned() {
    assert_eq!(
        "nls-remote-lease.v1+transfer=channel-5",
        channel_id("channel-5").channel_version(),
    );
}

// The handshake carries the suffixed version and every packet carries the bare
// one. A change that let the two converge would silently make a bare-version
// handshake acceptable, so the separation is asserted rather than assumed.
#[test]
fn channel_version_never_equals_the_bare_packet_version() {
    assert_ne!(VERSION, channel_id("channel-0").channel_version().as_str());
    assert_eq!(VERSION, ProtocolVersion.to_string());
}

#[test]
fn channel_version_round_trips_the_ordinal_bounds() {
    for ics20_channel in ["channel-0", "channel-1", "channel-65535"] {
        let id = channel_id(ics20_channel);
        assert_eq!(
            id,
            Ics20ChannelId::from_channel_version(&id.channel_version())
                .expect("a self-composed version must parse back"),
        );
    }
}

#[test]
fn ics20_channel_id_rejects_a_non_canonical_form() {
    // Above u16, leading zero, absent ordinal, non-digit tail, wrong prefix
    // case, empty, a sign, whitespace, and a multi-byte char in the ordinal
    // position — the last guards the digit scan against a char-boundary panic.
    for ics20_channel in [
        "channel-65536",
        "channel-01",
        "channel-",
        "channel-1a",
        "Channel-1",
        "",
        "channel-+1",
        "channel- 1",
        "channel-١",
    ] {
        assert!(
            matches!(
                ics20_channel.parse::<Ics20ChannelId>(),
                Err(Error::Ics20ChannelIdNonCanonical),
            ),
            "{ics20_channel:?} must be rejected as non-canonical",
        );
    }
}

#[test]
fn ics20_channel_id_rejects_an_over_long_form() {
    let over_cap = format!("channel-{}", "9".repeat(ICS20_CHANNEL_ID_MAX_BYTES));
    assert!(matches!(
        over_cap.parse::<Ics20ChannelId>(),
        Err(Error::Ics20ChannelIdTooLong {
            actual,
            max: ICS20_CHANNEL_ID_MAX_BYTES,
        }) if actual == over_cap.len(),
    ));
}

#[test]
fn channel_version_parse_rejects_a_malformed_version() {
    // The bare packet version, a missing tag, a foreign protocol, a suffix on a
    // foreign protocol, and a non-canonical id behind a well-formed prefix. A
    // doubled tag cannot fit the cap and is covered by
    // `channel_version_parse_rejects_an_over_long_version`.
    for version in [
        VERSION,
        "nls-remote-lease.v1channel-5",
        "ics20-1",
        "ics20-1+transfer=channel-5",
        "nls-remote-lease.v1+transfer=channel-01",
        "nls-remote-lease.v1+transfer=channel-65536",
    ] {
        assert!(
            matches!(
                Ics20ChannelId::from_channel_version(version),
                Err(Error::ChannelVersionMalformed),
            ),
            "{version:?} must be rejected as malformed",
        );
    }
}

#[test]
fn channel_version_parse_rejects_an_over_long_version() {
    // Two ways past the cap: bulk padding, and the doubled tag or suffix a
    // recomposition bug would produce — the grammar admits neither.
    for version in [
        "x".repeat(CHANNEL_VERSION_MAX_BYTES + 1),
        "nls-remote-lease.v1+transfer=+transfer=channel-5".to_string(),
        "nls-remote-lease.v1+transfer=channel-5+transfer=channel-6".to_string(),
    ] {
        assert!(
            matches!(
                Ics20ChannelId::from_channel_version(&version),
                Err(Error::ChannelVersionTooLong {
                    actual,
                    max: CHANNEL_VERSION_MAX_BYTES,
                }) if actual == version.len(),
            ),
            "{version:?} must be rejected as over-long",
        );
    }
}

// The JSON form is the rendered id, not the bare ordinal — the counterparty and
// every operator tool speak `channel-<n>`.
#[test]
fn ics20_channel_id_round_trip_is_the_rendered_string() {
    assert_round_trip_eq(r#""channel-5""#, &channel_id("channel-5"));
}

#[test]
fn ics20_channel_id_deserialize_non_canonical_rejected() {
    for bad_wire in [
        r#""channel-01""#,
        r#""channel-65536""#,
        r#""Channel-1""#,
        r#""5""#,
        r#""""#,
    ] {
        serde_json::from_str::<Ics20ChannelId>(bad_wire)
            .expect_err("a non-canonical id must fail deserialization");
    }
}

#[test]
fn ics20_channel_id_deserialize_non_string_rejected() {
    serde_json::from_str::<Ics20ChannelId>("5")
        .expect_err("the wire form is a string, never a bare number");
}

#[test]
fn channel_version_at_cap_passes_through_the_bound_unchanged() {
    let at_cap = channel_id("channel-65535").channel_version();
    assert_eq!(CHANNEL_VERSION_MAX_BYTES, at_cap.len());
    assert_eq!(at_cap.as_str(), bounded_channel_version(&at_cap));
}

#[test]
fn channel_version_bound_truncates_on_a_char_boundary() {
    // A multi-byte char straddling the cap must be dropped whole, not split.
    let over_cap = format!("{}€", "x".repeat(CHANNEL_VERSION_MAX_BYTES - 1));
    assert_eq!(
        "x".repeat(CHANNEL_VERSION_MAX_BYTES - 1),
        bounded_channel_version(&over_cap),
    );
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

fn operation_err(kind: RemoteErrorKind, message: &str) -> RemoteLeaseCallback {
    RemoteLeaseCallback::OperationErr(RemoteError::new(
        kind,
        RemoteErrorMessage::new(message).expect("a short message must be accepted"),
    ))
}

fn channel_id(ics20_channel: &str) -> Ics20ChannelId {
    ics20_channel
        .parse()
        .expect("a canonical channel id must be accepted")
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
    SwapParams::one(
        WireCoin::new(1000, Ticker::new("NLS")),
        WireCoin::new(42, Ticker::new("LPN")),
    )
    .expect("sample uses two distinct non-zero amounts")
}

fn sample_transfer_out_params() -> TransferOutParams {
    TransferOutParams::new(WireCoin::new(1000, Ticker::new("LC1")))
        .expect("sample uses a non-zero amount")
}
