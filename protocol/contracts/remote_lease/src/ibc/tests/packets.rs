use remote_lease::{
    callback::{
        OPERATION_ERR_MAX_BYTES, RemoteErrorMessage, RemoteLeaseCallback, RemoteOperationOutcome,
    },
    envelope::{LeaseAddrOnWire, PacketEnvelope},
    msg::{CloseLeaseParams, Operation},
    response::{
        CloseLeaseResponse, OpenLeaseResponse, RemoteLeaseId, Ticker, WireCoin,
        WireOperationResponse, WireSwapResponse,
    },
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
    let response = WireOperationResponse::CloseLease(CloseLeaseResponse {});
    let ack_bytes = StdAck::Success(cosmwasm_std::to_json_binary(&response).unwrap()).to_binary();

    let res = ibc_packet_ack(
        deps.as_mut(),
        testing::mock_env(),
        ack_msg(envelope_bytes, ack_bytes),
    )
    .unwrap();

    assert_dispatched_callback(
        &lease,
        RemoteLeaseCallback {
            nonce: 0,
            outcome: RemoteOperationOutcome::OperationOk(response),
        },
        &res.messages,
    );
}

#[test]
fn packet_ack_error_dispatches_operation_err() {
    const ERROR_MESSAGE: &str = "dex pool drained";

    let mut deps = deps_with_config();
    let lease = sdk_testing::user("lease-2");
    let envelope_bytes = encode_envelope(&envelope_with_close_lease(&lease));
    let ack_bytes = StdAck::error(ERROR_MESSAGE).to_binary();

    let res = ibc_packet_ack(
        deps.as_mut(),
        testing::mock_env(),
        ack_msg(envelope_bytes, ack_bytes),
    )
    .unwrap();

    assert_dispatched_callback(
        &lease,
        RemoteLeaseCallback {
            nonce: 0,
            outcome: RemoteOperationOutcome::OperationErr(
                RemoteErrorMessage::new(ERROR_MESSAGE).expect("test fixture under the cap"),
            ),
        },
        &res.messages,
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

    assert_dispatched_callback(
        &lease,
        RemoteLeaseCallback {
            nonce: 0,
            outcome: RemoteOperationOutcome::OperationTimeout,
        },
        &res.messages,
    );
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

// Content validation belongs to the addressee lease — a ticker outside the
// Nolus currency registry must pass through this controller untouched, or the
// relayer would retry the ack forever (issue #637).
#[test]
fn packet_ack_out_of_registry_ticker_dispatches_ok() {
    let mut deps = deps_with_config();
    let lease = sdk_testing::user("lease-alien-ticker");
    let envelope_bytes = encode_envelope(&envelope_with_close_lease(&lease));
    let response = WireOperationResponse::Swap(WireSwapResponse {
        amount_out: WireCoin::new(42, Ticker::new("NOT_IN_REGISTRY")),
    });
    let ack_bytes = StdAck::Success(cosmwasm_std::to_json_binary(&response).unwrap()).to_binary();

    let res = ibc_packet_ack(
        deps.as_mut(),
        testing::mock_env(),
        ack_msg(envelope_bytes, ack_bytes),
    )
    .unwrap();

    assert_dispatched_callback(
        &lease,
        RemoteLeaseCallback {
            nonce: 0,
            outcome: RemoteOperationOutcome::OperationOk(response),
        },
        &res.messages,
    );
}

// An undecodable success payload must not strand the packet: the ack commits a
// synthetic `OperationErr` (the named `UNDECODABLE_ACK_REASON`) so the lease
// receives a normal failure callback instead of an endless relayer retry.
#[test]
fn packet_ack_success_with_malformed_response_dispatches_synthetic_err() {
    let mut deps = deps_with_config();
    let lease = sdk_testing::user("lease-5");
    let envelope_bytes = encode_envelope(&envelope_with_close_lease(&lease));
    let ack_bytes = StdAck::Success(Binary::new(b"not-an-operation-response".to_vec())).to_binary();

    let res = ibc_packet_ack(
        deps.as_mut(),
        testing::mock_env(),
        ack_msg(envelope_bytes, ack_bytes),
    )
    .unwrap();

    assert_dispatched_callback(
        &lease,
        RemoteLeaseCallback {
            nonce: 0,
            outcome: RemoteOperationOutcome::OperationErr(RemoteErrorMessage::from_static(
                crate::ibc::UNDECODABLE_ACK_REASON,
            )),
        },
        &res.messages,
    );
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
    // or the `{ nonce, outcome }` envelope breaks the wire contract between
    // this controller and the lease-side `ExecuteMsg::RemoteLeaseCallback`
    // variant. The timeout fixture rides a zero nonce (`envelope_with_close_lease`).
    assert_eq!(
        br#"{"remote_lease_callback":{"nonce":0,"outcome":"operation_timeout"}}"#,
        msg.as_slice(),
    );
}

#[test]
fn packet_ack_malformed_lease_addr_in_envelope_errors() {
    let mut deps = deps_with_config();
    let envelope = PacketEnvelope {
        lease: LeaseAddrOnWire::new("NOT_BECH32!"),
        operation: Operation::CloseLease(CloseLeaseParams {}),
        version: ProtocolVersion,
        nonce: 0,
    };
    let envelope_bytes = cosmwasm_std::to_json_binary(&envelope).expect("envelope serialises");
    let response = WireOperationResponse::CloseLease(CloseLeaseResponse {});
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
    let envelope = PacketEnvelope {
        lease: LeaseAddrOnWire::new("NOT_BECH32!"),
        operation: Operation::CloseLease(CloseLeaseParams {}),
        version: ProtocolVersion,
        nonce: 0,
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

// Inbound-ack fixtures per ADR 0001 §3.5, pinned to the bytes the Solana-side
// Remote Lease App emits at `nolus-protocol/ibc-solray` main `73a8b163`. They
// live under `tests/fixtures/` as a frozen, byte-exact reference so cross-side
// drift between the Solana emitter and this controller is caught here even if
// the inline wire-types tests in the shared `remote_lease` crate stay green.
// See `tests/fixtures/README.md` for the emission path and regeneration steps.
//
// The OpenLease success ack's `remote_lease_id` is the lease's `LeaseAuthority`
// — the address the Cosmos side funds via ICS-20 (ibc-solray #486, ADR 0002
// §3.4 step 9), not the Lease PDA.
#[test]
fn fixture_stdack_success_open_lease_decodes_to_callback() {
    // A representative 32-byte PDA pubkey in canonical base58 (44 chars), in the
    // documented `remote_lease_id` range — a fixed stand-in, not a live address.
    const FIXTURE_REMOTE_LEASE_ID: &str = "CkymGXksQYqyYZrdvFTWwFhvMNBqENvKfKQN4e7CwBxF";
    const ACK_BYTES: &[u8] =
        include_bytes!("../../../tests/fixtures/stdack_success_open_lease.bin");
    let response = WireOperationResponse::OpenLease(OpenLeaseResponse {
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
        RemoteLeaseCallback {
            nonce: 0,
            outcome: RemoteOperationOutcome::OperationOk(response),
        },
        &res.messages,
    );
}

#[test]
fn fixture_stdack_error_decodes_to_callback() {
    // The Solana side prefixes every error-ack content with `ibc-solray: `
    // (ibc-solray `src/app/remote_lease/ack.rs`), so the full content the
    // controller receives — and lifts into `RemoteErrorMessage` — carries it.
    const FIXTURE_ERROR_MESSAGE: &str = "ibc-solray: dex pool drained";
    const ACK_BYTES: &[u8] = include_bytes!("../../../tests/fixtures/stdack_error.bin");

    let computed = StdAck::error(FIXTURE_ERROR_MESSAGE).to_binary();
    assert_eq!(
        ACK_BYTES,
        computed.as_slice(),
        "fixture must match the canonical wire shape"
    );

    let mut deps = deps_with_config();
    let lease = sdk_testing::user("lease-fixture-err");
    let envelope_bytes = encode_envelope(&envelope_with_close_lease(&lease));
    let res = ibc_packet_ack(
        deps.as_mut(),
        testing::mock_env(),
        ack_msg(envelope_bytes, Binary::new(ACK_BYTES.to_vec())),
    )
    .unwrap();
    assert_dispatched_callback(
        &lease,
        RemoteLeaseCallback {
            nonce: 0,
            outcome: RemoteOperationOutcome::OperationErr(
                RemoteErrorMessage::new(FIXTURE_ERROR_MESSAGE).expect("under the cap"),
            ),
        },
        &res.messages,
    );
}

// An over-cap counterparty error string must not strand the packet: the ack
// commits an `OperationErr` truncated to the byte cap, so the lease receives a
// normal failure callback instead of an endless relayer retry.
#[test]
fn packet_ack_oversized_error_message_dispatches_truncated_err() {
    let mut deps = deps_with_config();
    let lease = sdk_testing::user("lease-6");
    let envelope_bytes = encode_envelope(&envelope_with_close_lease(&lease));
    let oversized = "x".repeat(OPERATION_ERR_MAX_BYTES + 1);
    let ack_bytes = StdAck::error(oversized).to_binary();

    let res = ibc_packet_ack(
        deps.as_mut(),
        testing::mock_env(),
        ack_msg(envelope_bytes, ack_bytes),
    )
    .unwrap();

    assert_dispatched_callback(
        &lease,
        RemoteLeaseCallback {
            nonce: 0,
            outcome: RemoteOperationOutcome::OperationErr(
                RemoteErrorMessage::new("x".repeat(OPERATION_ERR_MAX_BYTES))
                    .expect("the truncated fixture is exactly at the cap"),
            ),
        },
        &res.messages,
    );
}

// AC (#636): on an acknowledgment the controller decodes the ORIGINAL outbound
// packet's envelope and forwards its `nonce` into the dispatched
// `RemoteLeaseCallback`, so the lease can correlate the ack to the exact packet
// it emitted.
#[test]
fn packet_ack_forwards_envelope_nonce_into_callback() {
    const NONCE: u64 = 7;

    let mut deps = deps_with_config();
    let lease = sdk_testing::user("lease-nonce-ack");
    let envelope_bytes = encode_envelope(&envelope_with_nonce(&lease, NONCE));
    let response = WireOperationResponse::CloseLease(CloseLeaseResponse {});
    let ack_bytes = StdAck::Success(cosmwasm_std::to_json_binary(&response).unwrap()).to_binary();

    let res = ibc_packet_ack(
        deps.as_mut(),
        testing::mock_env(),
        ack_msg(envelope_bytes, ack_bytes),
    )
    .unwrap();

    assert_dispatched_callback(
        &lease,
        RemoteLeaseCallback {
            nonce: NONCE,
            outcome: RemoteOperationOutcome::OperationOk(response),
        },
        &res.messages,
    );
}

// AC (#636): on a timeout the controller likewise decodes the original
// packet's envelope and forwards its `nonce` into the `OperationTimeout`
// callback.
#[test]
fn packet_timeout_forwards_envelope_nonce_into_callback() {
    const NONCE: u64 = 7;

    let mut deps = deps_with_config();
    let lease = sdk_testing::user("lease-nonce-timeout");
    let envelope_bytes = encode_envelope(&envelope_with_nonce(&lease, NONCE));

    let res = ibc_packet_timeout(
        deps.as_mut(),
        testing::mock_env(),
        timeout_msg(envelope_bytes),
    )
    .unwrap();

    assert_dispatched_callback(
        &lease,
        RemoteLeaseCallback {
            nonce: NONCE,
            outcome: RemoteOperationOutcome::OperationTimeout,
        },
        &res.messages,
    );
}

fn envelope_with_close_lease(lease: &Addr) -> PacketEnvelope {
    envelope_with_nonce(lease, 0)
}

fn envelope_with_nonce(lease: &Addr, nonce: u64) -> PacketEnvelope {
    PacketEnvelope {
        lease: LeaseAddrOnWire::new(lease.as_str()),
        operation: Operation::CloseLease(CloseLeaseParams {}),
        version: ProtocolVersion,
        nonce,
    }
}

fn encode_envelope(envelope: &PacketEnvelope) -> Binary {
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
    expected_callback: RemoteLeaseCallback,
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
