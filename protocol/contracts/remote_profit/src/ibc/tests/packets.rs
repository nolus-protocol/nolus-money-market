use remote_profit::{
    callback::{
        OPERATION_ERR_MAX_BYTES, RemoteErrorMessage, RemoteOperationOutcome, RemoteProfitCallback,
    },
    envelope::PacketEnvelope,
    msg::{CloseProfitParams, Operation},
    response::{
        CloseProfitResponse, OpenProfitResponse, RemoteProfitId, Ticker, WireCoin,
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
    profit_callback::ProfitExecuteMsg,
};

use super::{
    COUNTERPARTY_CHANNEL_ID, COUNTERPARTY_PORT_ID, LOCAL_CHANNEL_ID, LOCAL_PORT_ID,
    PROFIT_CONTRACT, deps_with_config,
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
    let envelope_bytes = encode_envelope(&envelope_with_close_profit());
    let response = WireOperationResponse::CloseProfit(CloseProfitResponse {});
    let ack_bytes = StdAck::Success(cosmwasm_std::to_json_binary(&response).unwrap()).to_binary();

    let res = ibc_packet_ack(
        deps.as_mut(),
        testing::mock_env(),
        ack_msg(envelope_bytes, ack_bytes),
    )
    .unwrap();

    assert_dispatched_callback(
        &sdk_testing::user(PROFIT_CONTRACT),
        RemoteProfitCallback {
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
    let envelope_bytes = encode_envelope(&envelope_with_close_profit());
    let ack_bytes = StdAck::error(ERROR_MESSAGE).to_binary();

    let res = ibc_packet_ack(
        deps.as_mut(),
        testing::mock_env(),
        ack_msg(envelope_bytes, ack_bytes),
    )
    .unwrap();

    assert_dispatched_callback(
        &sdk_testing::user(PROFIT_CONTRACT),
        RemoteProfitCallback {
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
    let envelope_bytes = encode_envelope(&envelope_with_close_profit());

    let res = ibc_packet_timeout(
        deps.as_mut(),
        testing::mock_env(),
        timeout_msg(envelope_bytes),
    )
    .unwrap();

    assert_dispatched_callback(
        &sdk_testing::user(PROFIT_CONTRACT),
        RemoteProfitCallback {
            nonce: 0,
            outcome: RemoteOperationOutcome::OperationTimeout,
        },
        &res.messages,
    );
}

// SECURITY (FM5): the ack path must NEVER produce or trust a wire-supplied
// `OperationTimeout`. `OperationTimeout` serialises as the bare string
// `"operation_timeout"`. We feed that exact value to the ack path two ways —
// as the success payload and as the (over-cap-irrelevant) error payload — and
// assert that NEITHER yields an `OperationTimeout` callback. The success arm
// fails to decode it as a `WireOperationResponse` (a hard error), and the error
// arm lifts it verbatim into an `OperationErr` string, never a timeout. The
// only producer of `OperationTimeout` is `ibc_packet_timeout`.
#[test]
fn packet_ack_never_yields_timeout_from_success_payload() {
    let mut deps = deps_with_config();
    let envelope_bytes = encode_envelope(&envelope_with_close_profit());
    // The literal wire encoding of `RemoteOperationOutcome::OperationTimeout`.
    let forged = Binary::new(br#""operation_timeout""#.to_vec());
    let ack_bytes = StdAck::Success(forged).to_binary();

    // A counterparty cannot smuggle a timeout through the success arm: the
    // bare string is not a valid `WireOperationResponse`, so the ack errors
    // out rather than dispatching a timeout.
    let err = ibc_packet_ack(
        deps.as_mut(),
        testing::mock_env(),
        ack_msg(envelope_bytes, ack_bytes),
    )
    .unwrap_err();
    assert!(matches!(err, Error::Std(_)), "got {err:?}");
}

#[test]
fn packet_ack_timeout_string_in_error_stays_operation_err() {
    let mut deps = deps_with_config();
    let envelope_bytes = encode_envelope(&envelope_with_close_profit());
    // Even if the counterparty authors the literal "operation_timeout" string
    // as the error message, the ack path lifts it into an `OperationErr`, never
    // an `OperationTimeout`.
    let ack_bytes = StdAck::error("operation_timeout").to_binary();

    let res = ibc_packet_ack(
        deps.as_mut(),
        testing::mock_env(),
        ack_msg(envelope_bytes, ack_bytes),
    )
    .unwrap();

    assert_dispatched_callback(
        &sdk_testing::user(PROFIT_CONTRACT),
        RemoteProfitCallback {
            nonce: 0,
            outcome: RemoteOperationOutcome::OperationErr(
                RemoteErrorMessage::new("operation_timeout").expect("under the cap"),
            ),
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
    let envelope_bytes = encode_envelope(&envelope_with_close_profit());
    let ack_bytes = Binary::new(b"not-a-std-ack".to_vec());

    let err = ibc_packet_ack(
        deps.as_mut(),
        testing::mock_env(),
        ack_msg(envelope_bytes, ack_bytes),
    )
    .unwrap_err();

    assert!(matches!(err, Error::Std(_)), "got {err:?}");
}

// Content validation belongs to the addressee profit — a ticker outside the
// Nolus currency registry must pass through this controller untouched, or the
// relayer would retry the ack forever (issue #637).
#[test]
fn packet_ack_out_of_registry_ticker_dispatches_ok() {
    let mut deps = deps_with_config();
    let envelope_bytes = encode_envelope(&envelope_with_close_profit());
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
        &sdk_testing::user(PROFIT_CONTRACT),
        RemoteProfitCallback {
            nonce: 0,
            outcome: RemoteOperationOutcome::OperationOk(response),
        },
        &res.messages,
    );
}

#[test]
fn packet_ack_success_with_malformed_response_errors() {
    let mut deps = deps_with_config();
    let envelope_bytes = encode_envelope(&envelope_with_close_profit());
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
    let envelope_bytes = encode_envelope(&envelope_with_close_profit());

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

    // Pin the JSON the profit contract must accept. Any drift in the enum tag
    // or the `{ nonce, outcome }` envelope breaks the wire contract between
    // this controller and the profit-side `ExecuteMsg::RemoteProfitCallback`
    // variant. The timeout fixture rides a zero nonce (`envelope_with_close_profit`).
    assert_eq!(
        br#"{"remote_profit_callback":{"nonce":0,"outcome":"operation_timeout"}}"#,
        msg.as_slice(),
    );
}

// The singleton envelope carries NO addressee, so a malformed-addressee path
// cannot exist; the equivalent malformed-envelope rejection is exercised by a
// version-pin mismatch — the `ProtocolVersion` ZST rejects any other value at
// deserialisation, never observed by business code.
#[test]
fn packet_ack_version_mismatch_in_envelope_errors() {
    let mut deps = deps_with_config();
    let envelope_bytes = Binary::new(
        br#"{"operation":{"close_profit":{}},"version":"nls-remote-profit.v2","nonce":0}"#.to_vec(),
    );
    let response = WireOperationResponse::CloseProfit(CloseProfitResponse {});
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
fn packet_timeout_version_mismatch_in_envelope_errors() {
    let mut deps = deps_with_config();
    let envelope_bytes = Binary::new(
        br#"{"operation":{"close_profit":{}},"version":"nls-remote-profit.v2","nonce":0}"#.to_vec(),
    );

    let err = ibc_packet_timeout(
        deps.as_mut(),
        testing::mock_env(),
        timeout_msg(envelope_bytes),
    )
    .unwrap_err();

    assert!(matches!(err, Error::Std(_)), "got {err:?}");
}

// Inbound-ack fixtures per ADR 0001 §3.5, pinned to the bytes the Solana-side
// Remote Profit App emits. They live under `tests/fixtures/` as a frozen,
// byte-exact reference so cross-side drift between the Solana emitter and this
// controller is caught here even if the inline wire-types tests in the shared
// `remote_profit` crate stay green. See `tests/fixtures/README.md` for the
// emission path and regeneration steps.
//
// The OpenProfit success ack's `remote_profit_id` is the singleton profit
// authority — the program-derived Solana address the Cosmos side funds via
// ICS-20 (ADR 0008), not a per-customer PDA.
#[test]
fn fixture_stdack_success_open_profit_decodes_to_callback() {
    // A representative 32-byte PDA pubkey in canonical base58 (44 chars), in the
    // documented `remote_profit_id` range — a fixed stand-in, not a live address.
    const FIXTURE_REMOTE_PROFIT_ID: &str = "CkymGXksQYqyYZrdvFTWwFhvMNBqENvKfKQN4e7CwBxF";
    const ACK_BYTES: &[u8] =
        include_bytes!("../../../tests/fixtures/stdack_success_open_profit.bin");
    let response = WireOperationResponse::OpenProfit(OpenProfitResponse {
        remote_profit_id: RemoteProfitId::new(FIXTURE_REMOTE_PROFIT_ID).expect("base58 fixture id"),
    });

    let computed = StdAck::Success(cosmwasm_std::to_json_binary(&response).unwrap()).to_binary();
    assert_eq!(
        ACK_BYTES,
        computed.as_slice(),
        "fixture must match the canonical wire shape"
    );

    let mut deps = deps_with_config();
    let envelope_bytes = encode_envelope(&envelope_with_close_profit());
    let res = ibc_packet_ack(
        deps.as_mut(),
        testing::mock_env(),
        ack_msg(envelope_bytes, Binary::new(ACK_BYTES.to_vec())),
    )
    .unwrap();
    assert_dispatched_callback(
        &sdk_testing::user(PROFIT_CONTRACT),
        RemoteProfitCallback {
            nonce: 0,
            outcome: RemoteOperationOutcome::OperationOk(response),
        },
        &res.messages,
    );
}

#[test]
fn fixture_stdack_error_decodes_to_callback() {
    // The Solana side prefixes every error-ack content with `ibc-solray: `
    // (ibc-solray `src/app/remote_profit/ack.rs`), so the full content the
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
    let envelope_bytes = encode_envelope(&envelope_with_close_profit());
    let res = ibc_packet_ack(
        deps.as_mut(),
        testing::mock_env(),
        ack_msg(envelope_bytes, Binary::new(ACK_BYTES.to_vec())),
    )
    .unwrap();
    assert_dispatched_callback(
        &sdk_testing::user(PROFIT_CONTRACT),
        RemoteProfitCallback {
            nonce: 0,
            outcome: RemoteOperationOutcome::OperationErr(
                RemoteErrorMessage::new(FIXTURE_ERROR_MESSAGE).expect("under the cap"),
            ),
        },
        &res.messages,
    );
}

#[test]
fn packet_ack_oversized_error_message_errors() {
    let mut deps = deps_with_config();
    let envelope_bytes = encode_envelope(&envelope_with_close_profit());
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

// AC (#636): on an acknowledgment the controller decodes the ORIGINAL outbound
// packet's envelope and forwards its `nonce` into the dispatched
// `RemoteProfitCallback`, so the profit can correlate the ack to the exact
// packet it emitted.
#[test]
fn packet_ack_forwards_envelope_nonce_into_callback() {
    const NONCE: u64 = 7;

    let mut deps = deps_with_config();
    let envelope_bytes = encode_envelope(&envelope_with_nonce(NONCE));
    let response = WireOperationResponse::CloseProfit(CloseProfitResponse {});
    let ack_bytes = StdAck::Success(cosmwasm_std::to_json_binary(&response).unwrap()).to_binary();

    let res = ibc_packet_ack(
        deps.as_mut(),
        testing::mock_env(),
        ack_msg(envelope_bytes, ack_bytes),
    )
    .unwrap();

    assert_dispatched_callback(
        &sdk_testing::user(PROFIT_CONTRACT),
        RemoteProfitCallback {
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
    let envelope_bytes = encode_envelope(&envelope_with_nonce(NONCE));

    let res = ibc_packet_timeout(
        deps.as_mut(),
        testing::mock_env(),
        timeout_msg(envelope_bytes),
    )
    .unwrap();

    assert_dispatched_callback(
        &sdk_testing::user(PROFIT_CONTRACT),
        RemoteProfitCallback {
            nonce: NONCE,
            outcome: RemoteOperationOutcome::OperationTimeout,
        },
        &res.messages,
    );
}

fn envelope_with_close_profit() -> PacketEnvelope {
    envelope_with_nonce(0)
}

fn envelope_with_nonce(nonce: u64) -> PacketEnvelope {
    PacketEnvelope {
        operation: Operation::CloseProfit(CloseProfitParams {}),
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
    expected_profit: &Addr,
    expected_callback: RemoteProfitCallback,
    messages: &[SubMsg],
) {
    assert_eq!(1, messages.len(), "expected one dispatched message");
    match &messages[0].msg {
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr,
            msg,
            funds,
        }) => {
            assert_eq!(expected_profit.as_str(), contract_addr);
            assert!(funds.is_empty(), "callback must carry no funds");
            let expected_msg = cosmwasm_std::to_json_binary(
                &ProfitExecuteMsg::RemoteProfitCallback(expected_callback),
            )
            .expect("expected callback must serialise");
            assert_eq!(&expected_msg, msg);
        }
        other => panic!("expected WasmMsg::Execute, got {other:?}"),
    }
}
