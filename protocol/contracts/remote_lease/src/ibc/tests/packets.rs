use currencies::{LeaseGroup, Lpns as LpnGroup, PaymentGroup};
use remote_lease::{
    callback::{
        OPERATION_ERR_MAX_BYTES, RemoteError, RemoteErrorKind, RemoteErrorMessage,
        RemoteLeaseCallback,
    },
    envelope::{LeaseAddrOnWire, PacketEnvelope},
    msg::{CloseLeaseParams, Operation},
    response::{CloseLeaseResponse, OpenLeaseResponse, OperationResponse, RemoteLeaseId},
    version::ProtocolVersion,
};
use sdk::{
    cosmwasm_std::{
        self, Addr, Binary, CosmosMsg, IbcAcknowledgement, IbcEndpoint, IbcPacket, IbcPacketAckMsg,
        IbcPacketReceiveMsg, IbcPacketTimeoutMsg, IbcTimeout, StdAck, SubMsg, Timestamp, WasmMsg,
        testing,
    },
    testing as sdk_testing,
};

use crate::{
    error::Error,
    ibc::{ibc_packet_ack, ibc_packet_receive, ibc_packet_timeout},
    lease_callback::LeaseExecuteMsg,
};

use super::{
    COUNTERPARTY_CHANNEL_ID, COUNTERPARTY_PORT_ID, LOCAL_CHANNEL_ID, LOCAL_PORT_ID,
    deps_with_config,
};

type PacketEnvelopeT = PacketEnvelope<LeaseGroup, LpnGroup, PaymentGroup>;

#[test]
fn packet_receive_returns_error_ack() {
    let mut deps = deps_with_config();
    let packet = IbcPacket::new(
        Binary::new(b"anything".to_vec()),
        IbcEndpoint {
            port_id: COUNTERPARTY_PORT_ID.into(),
            channel_id: COUNTERPARTY_CHANNEL_ID.into(),
        },
        IbcEndpoint {
            port_id: LOCAL_PORT_ID.into(),
            channel_id: LOCAL_CHANNEL_ID.into(),
        },
        1,
        IbcTimeout::with_timestamp(Timestamp::from_seconds(1)),
    );
    let relayer = sdk_testing::user("relayer");
    let msg = IbcPacketReceiveMsg::new(packet, relayer);

    let res = ibc_packet_receive(deps.as_mut(), testing::mock_env(), msg).unwrap();
    let ack: StdAck =
        sdk::cosmwasm_std::from_json(res.acknowledgement.expect("ack present")).unwrap();
    assert!(matches!(ack, StdAck::Error(_)));
    assert!(res.messages.is_empty());
}

#[test]
fn packet_ack_success_dispatches_operation_ok() {
    let mut deps = deps_with_config();
    let lease = sdk_testing::user("lease-1");
    let envelope_bytes = encode_envelope(&envelope_with_close_lease(&lease));
    let response = OperationResponse::CloseLease(CloseLeaseResponse {});
    let ack_bytes = StdAck::Success(cosmwasm_std::to_json_binary(&response).unwrap()).to_binary();

    let res = ibc_packet_ack(
        deps.as_mut(),
        testing::mock_env(),
        ack_msg(envelope_bytes, ack_bytes),
    )
    .unwrap();

    assert_dispatched_callback(
        &lease,
        RemoteLeaseCallback::OperationOk(response),
        &res.messages,
    );
}

#[test]
fn packet_ack_error_dispatches_operation_err() {
    assert_error_ack_dispatches(
        RemoteErrorKind::Permanent,
        "dex pool drained",
        "lease-2",
        "[permanent] dex pool drained",
    );
}

#[test]
fn packet_ack_min_out_error_dispatches_min_out_kind() {
    assert_error_ack_dispatches(
        RemoteErrorKind::MinOutUnmet,
        "ibc-solray: credit below min",
        "lease-2-min-out",
        "[min_out_unmet] ibc-solray: credit below min",
    );
}

#[test]
fn packet_ack_code_only_error_dispatches_an_empty_message() {
    assert_error_ack_dispatches(
        RemoteErrorKind::Transient,
        "",
        "lease-2-bare",
        "[transient]",
    );
}

// A non-conforming acknowledgement is rejected, not coerced into some default
// kind: guessing would invent a meaning the counterparty never sent and then
// route funds with it. The cost is deliberate — an `Err` here reverts
// `ibc_packet_ack` and the relayer redelivers until the fault is deployed away.
#[test]
fn packet_ack_non_conforming_error_code_errors() {
    for ack in [
        "dex pool drained",
        "[min_out_unmet dex pool drained",
        "min_out_unmet] dex pool drained",
        "[] dex pool drained",
        "[MIN_OUT_UNMET] dex pool drained",
        "[future_class] dex pool drained",
    ] {
        let mut deps = deps_with_config();
        let lease = sdk_testing::user("lease-non-conforming");
        let envelope_bytes = encode_envelope(&envelope_with_close_lease(&lease));

        let err = ibc_packet_ack(
            deps.as_mut(),
            testing::mock_env(),
            ack_msg(envelope_bytes, StdAck::error(ack).to_binary()),
        )
        .unwrap_err();

        assert!(
            matches!(err, Error::RemoteCallback(_)),
            "{ack:?} must be rejected, got {err:?}",
        );
    }
}

// The cap applies to the prose once the frame is stripped, so an
// acknowledgement over-long only by its frame is accepted where it previously
// would not have been.
#[test]
fn packet_ack_error_at_cap_after_stripping_dispatches() {
    let prose = "x".repeat(OPERATION_ERR_MAX_BYTES);
    let framed = RemoteError::format_ack(RemoteErrorKind::Transient, &prose);
    assert!(framed.len() > OPERATION_ERR_MAX_BYTES);

    assert_error_ack_dispatches(
        RemoteErrorKind::Transient,
        &prose,
        "lease-2-at-cap",
        &framed,
    );
}

#[test]
fn packet_timeout_dispatches_operation_timeout() {
    let mut deps = deps_with_config();
    let lease = sdk_testing::user("lease-3");
    let envelope_bytes = encode_envelope(&envelope_with_close_lease(&lease));

    let res = ibc_packet_timeout(
        deps.as_mut(),
        testing::mock_env(),
        timeout_msg(envelope_bytes),
    )
    .unwrap();

    assert_dispatched_callback(&lease, RemoteLeaseCallback::OperationTimeout, &res.messages);
}

#[test]
fn packet_ack_malformed_envelope_errors() {
    let mut deps = deps_with_config();
    let envelope_bytes = Binary::new(b"not-an-envelope".to_vec());
    let ack_bytes = StdAck::Success(Binary::new(b"{}".to_vec())).to_binary();

    let err = ibc_packet_ack(
        deps.as_mut(),
        testing::mock_env(),
        ack_msg(envelope_bytes, ack_bytes),
    )
    .unwrap_err();

    assert!(matches!(err, Error::Std(_)), "got {err:?}");
}

#[test]
fn packet_timeout_malformed_envelope_errors() {
    let mut deps = deps_with_config();
    let envelope_bytes = Binary::new(b"not-an-envelope".to_vec());

    let err = ibc_packet_timeout(
        deps.as_mut(),
        testing::mock_env(),
        timeout_msg(envelope_bytes),
    )
    .unwrap_err();

    assert!(matches!(err, Error::Std(_)), "got {err:?}");
}

#[test]
fn packet_ack_malformed_acknowledgement_errors() {
    let mut deps = deps_with_config();
    let lease = sdk_testing::user("lease-4");
    let envelope_bytes = encode_envelope(&envelope_with_close_lease(&lease));
    let ack_bytes = Binary::new(b"not-a-std-ack".to_vec());

    let err = ibc_packet_ack(
        deps.as_mut(),
        testing::mock_env(),
        ack_msg(envelope_bytes, ack_bytes),
    )
    .unwrap_err();

    assert!(matches!(err, Error::Std(_)), "got {err:?}");
}

#[test]
fn packet_ack_success_with_malformed_response_errors() {
    let mut deps = deps_with_config();
    let lease = sdk_testing::user("lease-5");
    let envelope_bytes = encode_envelope(&envelope_with_close_lease(&lease));
    let ack_bytes = StdAck::Success(Binary::new(b"not-an-operation-response".to_vec())).to_binary();

    let err = ibc_packet_ack(
        deps.as_mut(),
        testing::mock_env(),
        ack_msg(envelope_bytes, ack_bytes),
    )
    .unwrap_err();

    assert!(matches!(err, Error::Std(_)), "got {err:?}");
}

#[test]
fn dispatched_callback_wire_shape_pinned() {
    let mut deps = deps_with_config();
    let lease = sdk_testing::user("lease-wire");
    let envelope_bytes = encode_envelope(&envelope_with_close_lease(&lease));

    let res = ibc_packet_timeout(
        deps.as_mut(),
        testing::mock_env(),
        timeout_msg(envelope_bytes),
    )
    .unwrap();

    let SubMsg { msg, .. } = res.messages.into_iter().next().expect("one message");
    let CosmosMsg::Wasm(WasmMsg::Execute { msg, .. }) = msg else {
        panic!("expected WasmMsg::Execute, got {msg:?}");
    };

    // Pin the JSON the lease contract must accept. Any drift in the enum tag
    // breaks the wire contract between this controller and the lease-side
    // `ExecuteMsg::RemoteLeaseCallback` variant.
    assert_eq!(
        br#"{"remote_lease_callback":"operation_timeout"}"#,
        msg.as_slice(),
    );
}

#[test]
fn packet_ack_malformed_lease_addr_in_envelope_errors() {
    let mut deps = deps_with_config();
    let envelope = PacketEnvelopeT {
        lease: LeaseAddrOnWire::new("NOT_BECH32!"),
        operation: Operation::CloseLease(CloseLeaseParams {}),
        version: ProtocolVersion,
    };
    let envelope_bytes = cosmwasm_std::to_json_binary(&envelope).expect("envelope serialises");
    let response: OperationResponse<PaymentGroup> =
        OperationResponse::CloseLease(CloseLeaseResponse {});
    let ack_bytes = StdAck::Success(cosmwasm_std::to_json_binary(&response).unwrap()).to_binary();

    let err = ibc_packet_ack(
        deps.as_mut(),
        testing::mock_env(),
        ack_msg(envelope_bytes, ack_bytes),
    )
    .unwrap_err();

    assert!(matches!(err, Error::Std(_)), "got {err:?}");
}

#[test]
fn packet_timeout_malformed_lease_addr_in_envelope_errors() {
    let mut deps = deps_with_config();
    let envelope = PacketEnvelopeT {
        lease: LeaseAddrOnWire::new("NOT_BECH32!"),
        operation: Operation::CloseLease(CloseLeaseParams {}),
        version: ProtocolVersion,
    };
    let envelope_bytes = cosmwasm_std::to_json_binary(&envelope).expect("envelope serialises");

    let err = ibc_packet_timeout(
        deps.as_mut(),
        testing::mock_env(),
        timeout_msg(envelope_bytes),
    )
    .unwrap_err();

    assert!(matches!(err, Error::Std(_)), "got {err:?}");
}

// Inbound-ack fixtures per ADR 0001 §3.5. The bytes the controller must accept
// from the Solana counterparty live under `tests/fixtures/` so they survive as
// a frozen, byte-exact reference even if the inline wire-types tests in the
// shared `remote_lease` crate ever drift. See `tests/fixtures/README.md` for
// regeneration steps and the placeholder note (Solana-emitted bytes are TBD).
#[test]
fn fixture_stdack_success_open_lease_decodes_to_callback() {
    const FIXTURE_REMOTE_LEASE_ID: &str = "So1RayF1xtureLease1";
    const ACK_BYTES: &[u8] =
        include_bytes!("../../../tests/fixtures/stdack_success_open_lease.bin");
    let response = OperationResponse::OpenLease(OpenLeaseResponse {
        remote_lease_id: RemoteLeaseId::new(FIXTURE_REMOTE_LEASE_ID).expect("base58 fixture id"),
    });

    let computed = StdAck::Success(cosmwasm_std::to_json_binary(&response).unwrap()).to_binary();
    assert_eq!(
        ACK_BYTES,
        computed.as_slice(),
        "fixture must match the canonical wire shape"
    );

    let mut deps = deps_with_config();
    let lease = sdk_testing::user("lease-fixture");
    let envelope_bytes = encode_envelope(&envelope_with_close_lease(&lease));
    let res = ibc_packet_ack(
        deps.as_mut(),
        testing::mock_env(),
        ack_msg(envelope_bytes, Binary::new(ACK_BYTES.to_vec())),
    )
    .unwrap();
    assert_dispatched_callback(
        &lease,
        RemoteLeaseCallback::OperationOk(response),
        &res.messages,
    );
}

// Both error fixtures below are counterparty output, not authored here: the
// bytes are what `ibc-solray`'s `app::remote_lease::ack::error` emitted at
// `ec40d50d` for a real `api::Error` value, captured verbatim. That is what makes
// the byte assertion a cross-repo agreement rather than a restatement of this
// crate's own rendering — it fails if either end changes how the frame, the
// provenance prefix or the ICS-26 envelope is composed.
//
// To regenerate: for the chosen `Error` variant, print
// `ack::error(&e, OPERATION_ERR_MAX_BYTES).as_bytes()` from that tree and
// replace the `.bin` plus the prose below.
#[test]
fn fixture_stdack_error_decodes_to_callback() {
    // `Error::SwapAmountInMismatch { expected: 1_000, got: 999 }` — a
    // `Disposition::PermanentInput` variant, so it frames as `permanent`.
    const FIXTURE_ACK: &str =
        "[permanent] ibc-solray: Swap amount_in mismatch: expected '1000', got '999'";
    const ACK_BYTES: &[u8] = include_bytes!("../../../tests/fixtures/stdack_error.bin");

    let computed = StdAck::error(FIXTURE_ACK).to_binary();
    assert_eq!(
        ACK_BYTES,
        computed.as_slice(),
        "fixture must match the canonical wire shape"
    );

    assert_error_ack_dispatches(
        RemoteErrorKind::Permanent,
        "ibc-solray: Swap amount_in mismatch: expected '1000', got '999'",
        "lease-fixture-err",
        FIXTURE_ACK,
    );
}

#[test]
fn fixture_stdack_error_min_out_decodes_to_callback() {
    // `Error::SwapPostBalanceCreditBelowMin { min_required: 42, got: 41 }` — the
    // post-CPI floor check, which the counterparty frames as `min_out_unmet`
    // despite its `Disposition::Stale` class.
    const FIXTURE_ACK: &str = "[min_out_unmet] ibc-solray: Swap post-balance credit below required min: min_required '42', got '41'";
    const ACK_BYTES: &[u8] = include_bytes!("../../../tests/fixtures/stdack_error_min_out.bin");

    let computed = StdAck::error(FIXTURE_ACK).to_binary();
    assert_eq!(
        ACK_BYTES,
        computed.as_slice(),
        "fixture must match the canonical wire shape"
    );

    assert_error_ack_dispatches(
        RemoteErrorKind::MinOutUnmet,
        "ibc-solray: Swap post-balance credit below required min: min_required '42', got '41'",
        "lease-fixture-err-min-out",
        FIXTURE_ACK,
    );
}

#[test]
fn packet_ack_oversized_error_message_errors() {
    let mut deps = deps_with_config();
    let lease = sdk_testing::user("lease-6");
    let envelope_bytes = encode_envelope(&envelope_with_close_lease(&lease));
    let oversized = "x".repeat(OPERATION_ERR_MAX_BYTES + 1);
    let ack_bytes = StdAck::error(oversized).to_binary();

    let err = ibc_packet_ack(
        deps.as_mut(),
        testing::mock_env(),
        ack_msg(envelope_bytes, ack_bytes),
    )
    .unwrap_err();

    assert!(matches!(err, Error::RemoteCallback(_)), "got {err:?}");
}

// Drives the real `ibc_packet_ack`, so each caller proves both the
// classification and that the handler committed the acknowledgement.
fn assert_error_ack_dispatches(
    expected_kind: RemoteErrorKind,
    expected_message: &str,
    lease_id: &str,
    ack: &str,
) {
    let mut deps = deps_with_config();
    let lease = sdk_testing::user(lease_id);
    let envelope_bytes = encode_envelope(&envelope_with_close_lease(&lease));

    let res = ibc_packet_ack(
        deps.as_mut(),
        testing::mock_env(),
        ack_msg(envelope_bytes, StdAck::error(ack).to_binary()),
    )
    .unwrap();

    assert_dispatched_callback(
        &lease,
        RemoteLeaseCallback::OperationErr(RemoteError::new(
            expected_kind,
            RemoteErrorMessage::new(expected_message).expect("test fixture under the cap"),
        )),
        &res.messages,
    );
}

fn envelope_with_close_lease(lease: &Addr) -> PacketEnvelopeT {
    PacketEnvelopeT {
        lease: LeaseAddrOnWire::new(lease.as_str()),
        operation: Operation::CloseLease(CloseLeaseParams {}),
        version: ProtocolVersion,
    }
}

fn encode_envelope(envelope: &PacketEnvelopeT) -> Binary {
    cosmwasm_std::to_json_binary(envelope).expect("envelope must serialise")
}

fn ack_msg(envelope_bytes: Binary, ack_bytes: Binary) -> IbcPacketAckMsg {
    IbcPacketAckMsg::new(
        IbcAcknowledgement::new(ack_bytes),
        outbound_packet(envelope_bytes),
        sdk_testing::user("relayer"),
    )
}

fn timeout_msg(envelope_bytes: Binary) -> IbcPacketTimeoutMsg {
    IbcPacketTimeoutMsg::new(
        outbound_packet(envelope_bytes),
        sdk_testing::user("relayer"),
    )
}

fn outbound_packet(data: Binary) -> IbcPacket {
    IbcPacket::new(
        data,
        IbcEndpoint {
            port_id: LOCAL_PORT_ID.into(),
            channel_id: LOCAL_CHANNEL_ID.into(),
        },
        IbcEndpoint {
            port_id: COUNTERPARTY_PORT_ID.into(),
            channel_id: COUNTERPARTY_CHANNEL_ID.into(),
        },
        1,
        IbcTimeout::with_timestamp(Timestamp::from_seconds(1)),
    )
}

fn assert_dispatched_callback(
    expected_lease: &Addr,
    expected_callback: RemoteLeaseCallback<PaymentGroup>,
    messages: &[SubMsg],
) {
    assert_eq!(1, messages.len(), "expected one dispatched message");
    match &messages[0].msg {
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr,
            msg,
            funds,
        }) => {
            assert_eq!(expected_lease.as_str(), contract_addr);
            assert!(funds.is_empty(), "callback must carry no funds");
            let expected_msg = cosmwasm_std::to_json_binary(&LeaseExecuteMsg::RemoteLeaseCallback(
                expected_callback,
            ))
            .expect("expected callback must serialise");
            assert_eq!(&expected_msg, msg);
        }
        other => panic!("expected WasmMsg::Execute, got {other:?}"),
    }
}
