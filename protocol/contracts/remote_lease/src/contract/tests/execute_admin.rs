use platform::contract::{Code, CodeId};
use sdk::{
    cosmos_sdk_proto::prost::Message as _,
    cosmwasm_ext::{CosmosMsg, Response as CwResponse, SubMsg},
    cosmwasm_std::{AnyMsg, DepsMut, IbcMsg, Uint64, testing},
    ibc_proto::ibc::core::channel::v1::{
        MsgChannelOpenInit, Order as ProtoOrder, State as ProtoState,
    },
};

use crate::{
    api::ExecuteMsg,
    contract::{execute, instantiate},
    error::Error,
    state::Channel,
};

use super::{
    ADMIN, CONNECTION_ID, COUNTERPARTY_PORT_ID, ExecuteMsgT, ICS20_CHANNEL_REMOTE, LEASE_CODE_ID,
    LOCAL_CHANNEL_ID, NON_ADMIN, PROPOSED_VERSION, deps, ics20_channel_remote, instantiate_default,
    instantiate_msg, query_channel, query_config, sender, store_closing_channel,
    store_init_accepted, store_open_channel, store_proposal,
};

#[test]
fn new_lease_code_admin_succeeds() {
    let mut deps = deps();
    instantiate(
        deps.as_mut(),
        testing::mock_env(),
        sender(super::CREATOR),
        instantiate_msg(),
    )
    .unwrap();

    let new_code = Code::unchecked(LEASE_CODE_ID + 5);
    let res = execute(
        deps.as_mut(),
        testing::mock_env(),
        sender(ADMIN),
        ExecuteMsg::NewLeaseCode {
            lease_code: new_code,
        },
    )
    .unwrap();
    assert_eq!(0, res.messages.len());

    let config = query_config(deps.as_ref());
    assert_eq!(Uint64::from(CodeId::from(new_code)), config.lease_code_id);
}

// Decodes the proto payload rather than asserting it is non-empty: the
// handshake version now carries the paired transfer channel, and those bytes
// are the only place that proposal reaches the counterparty.
#[test]
fn open_channel_admin_emits_any_msg() {
    const CHANNEL_OPEN_INIT: &str = "/ibc.core.channel.v1.MsgChannelOpenInit";

    let mut deps = deps();
    instantiate_default(deps.as_mut());

    let res = execute(
        deps.as_mut(),
        testing::mock_env(),
        sender(ADMIN),
        ExecuteMsg::OpenChannel {
            ics20_channel_remote: ics20_channel_remote(ICS20_CHANNEL_REMOTE),
        },
    )
    .unwrap();
    assert_eq!(1, res.messages.len());
    let open_init = match &res.messages[0] {
        SubMsg {
            msg: CosmosMsg::Any(AnyMsg { type_url, value }),
            ..
        } => {
            assert_eq!(CHANNEL_OPEN_INIT, type_url);
            MsgChannelOpenInit::decode(value.as_slice()).expect("a well-formed open-init payload")
        }
        other => panic!("expected CosmosMsg::Any, got {other:?}"),
    };

    let env = testing::mock_env();
    assert_eq!(format!("wasm.{}", env.contract.address), open_init.port_id);
    assert_eq!(env.contract.address.as_str(), open_init.signer);

    let channel = open_init.channel.expect("open-init carries a channel");
    assert_eq!(PROPOSED_VERSION, channel.version);
    assert_eq!(ProtoState::Init as i32, channel.state);
    assert_eq!(ProtoOrder::Unordered as i32, channel.ordering);
    assert_eq!(vec![CONNECTION_ID.to_string()], channel.connection_hops);
    let counterparty = channel
        .counterparty
        .expect("open-init names a counterparty");
    assert_eq!(COUNTERPARTY_PORT_ID, counterparty.port_id);
    assert_eq!("", counterparty.channel_id);

    assert_eq!(
        Channel::Proposed {
            ics20_channel_remote: ics20_channel_remote(ICS20_CHANNEL_REMOTE),
        },
        Channel::may_load(&deps.storage)
            .unwrap()
            .expect("the proposal outlives the emitted message"),
    );
}

// The typed field is the whole defence: a non-canonical id cannot be built, so
// it never reaches `execute` and the controller carries no error for it.
#[test]
fn open_channel_non_canonical_id_rejected_at_decode() {
    for ics20_channel_remote in ["channel-65536", "channel-01", "channel-", "Channel-1"] {
        let raw =
            format!(r#"{{"open_channel":{{"ics20_channel_remote":"{ics20_channel_remote}"}}}}"#);
        sdk::cosmwasm_std::from_json::<ExecuteMsgT>(raw.as_bytes())
            .expect_err("a non-canonical channel id must fail ExecuteMsg deserialization");
    }
}

#[test]
fn open_channel_canonical_id_decodes() {
    let msg: ExecuteMsgT =
        sdk::cosmwasm_std::from_json(br#"{"open_channel":{"ics20_channel_remote":"channel-5"}}"#)
            .expect("a canonical channel id must decode");
    assert_eq!(
        ExecuteMsg::OpenChannel {
            ics20_channel_remote: ics20_channel_remote(ICS20_CHANNEL_REMOTE),
        },
        msg,
    );
}

// A handshake in flight is never replaced silently — the operator abandons it
// explicitly. Both in-flight phases refuse a fresh proposal.
#[test]
fn open_channel_while_handshake_in_flight_rejected() {
    for store_in_flight in [
        store_proposal as fn(DepsMut<'_>),
        store_init_accepted as fn(DepsMut<'_>),
    ] {
        let mut deps = deps();
        instantiate_default(deps.as_mut());
        store_in_flight(deps.as_mut());

        let err = open_channel_by(deps.as_mut(), ADMIN, "channel-6").unwrap_err();
        assert!(matches!(err, Error::ProposalPending), "got {err:?}");

        // The in-flight handshake is left exactly as it was.
        assert_eq!(
            PROPOSED_VERSION,
            Channel::may_load(&deps.storage).unwrap().unwrap().version(),
        );
    }
}

#[test]
fn open_channel_non_admin_rejected() {
    let mut deps = deps();
    instantiate_default(deps.as_mut());
    let err = execute(
        deps.as_mut(),
        testing::mock_env(),
        sender(NON_ADMIN),
        ExecuteMsg::OpenChannel {
            ics20_channel_remote: ics20_channel_remote(ICS20_CHANNEL_REMOTE),
        },
    )
    .unwrap_err();
    assert!(matches!(err, Error::Unauthorized(_)), "got {err:?}");

    assert!(Channel::may_load(&deps.storage).unwrap().is_none());
}

#[test]
fn open_channel_when_channel_exists_rejected() {
    let mut deps = deps();
    instantiate_default(deps.as_mut());
    store_open_channel(deps.as_mut());

    let err = execute(
        deps.as_mut(),
        testing::mock_env(),
        sender(ADMIN),
        ExecuteMsg::OpenChannel {
            ics20_channel_remote: ics20_channel_remote(ICS20_CHANNEL_REMOTE),
        },
    )
    .unwrap_err();
    assert!(matches!(err, Error::ChannelAlreadyExists), "got {err:?}");
}

// ---------------------------------------------------------------------------
// CancelChannelProposal — the operator's escape from a stalled handshake
// ---------------------------------------------------------------------------

#[test]
fn cancel_proposal_clears_an_in_flight_handshake() {
    for store_in_flight in [
        store_proposal as fn(DepsMut<'_>),
        store_init_accepted as fn(DepsMut<'_>),
    ] {
        let mut deps = deps();
        instantiate_default(deps.as_mut());
        store_in_flight(deps.as_mut());

        let res = cancel_by(deps.as_mut(), ADMIN).expect("an in-flight handshake is cancellable");
        assert_eq!(0, res.messages.len(), "cancelling is local-only");

        assert!(Channel::may_load(&deps.storage).unwrap().is_none());

        // Cancelling is what frees the controller to propose again.
        open_channel_by(deps.as_mut(), ADMIN, "channel-6")
            .expect("a cancelled handshake frees a fresh proposal");
    }
}

#[test]
fn cancel_proposal_when_established_rejected() {
    let mut deps = deps();
    instantiate_default(deps.as_mut());
    store_open_channel(deps.as_mut());

    let err = cancel_by(deps.as_mut(), ADMIN).unwrap_err();
    assert!(matches!(err, Error::ChannelAlreadyExists), "got {err:?}");

    assert!(Channel::may_load(&deps.storage).unwrap().is_some());
}

#[test]
fn cancel_proposal_when_nothing_pending_rejected() {
    let mut deps = deps();
    instantiate_default(deps.as_mut());

    let err = cancel_by(deps.as_mut(), ADMIN).unwrap_err();
    assert!(matches!(err, Error::NoProposalToCancel), "got {err:?}");
}

#[test]
fn cancel_proposal_non_admin_rejected() {
    let mut deps = deps();
    instantiate_default(deps.as_mut());
    store_proposal(deps.as_mut());

    let err = cancel_by(deps.as_mut(), NON_ADMIN).unwrap_err();
    assert!(matches!(err, Error::Unauthorized(_)), "got {err:?}");

    assert!(Channel::may_load(&deps.storage).unwrap().is_some());
}

#[test]
fn close_channel_admin_transitions_state_and_emits_close() {
    let mut deps = deps();
    instantiate_default(deps.as_mut());
    store_open_channel(deps.as_mut());

    let res = execute(
        deps.as_mut(),
        testing::mock_env(),
        sender(ADMIN),
        ExecuteMsg::CloseChannel(),
    )
    .unwrap();
    assert_eq!(1, res.messages.len());
    assert!(matches!(
        &res.messages[0].msg,
        CosmosMsg::Ibc(IbcMsg::CloseChannel { channel_id }) if channel_id == LOCAL_CHANNEL_ID
    ));

    assert!(matches!(
        query_channel(deps.as_ref()).channel.unwrap(),
        crate::api::ChannelInfo::Established {
            state: crate::api::ChannelStateResponse::Closing,
            ..
        },
    ));
}

#[test]
fn close_channel_non_admin_rejected() {
    let mut deps = deps();
    instantiate_default(deps.as_mut());
    store_open_channel(deps.as_mut());

    let err = execute(
        deps.as_mut(),
        testing::mock_env(),
        sender(NON_ADMIN),
        ExecuteMsg::CloseChannel(),
    )
    .unwrap_err();
    assert!(matches!(err, Error::Unauthorized(_)), "got {err:?}");
}

#[test]
fn close_channel_when_absent_rejected() {
    let mut deps = deps();
    instantiate_default(deps.as_mut());

    let err = execute(
        deps.as_mut(),
        testing::mock_env(),
        sender(ADMIN),
        ExecuteMsg::CloseChannel(),
    )
    .unwrap_err();
    assert!(matches!(err, Error::ChannelNotOpen), "got {err:?}");
}

#[test]
fn close_channel_when_already_closing_rejected() {
    let mut deps = deps();
    instantiate_default(deps.as_mut());
    store_closing_channel(deps.as_mut());

    let err = execute(
        deps.as_mut(),
        testing::mock_env(),
        sender(ADMIN),
        ExecuteMsg::CloseChannel(),
    )
    .unwrap_err();
    assert!(matches!(err, Error::ChannelNotOperational), "got {err:?}");
}

#[test]
fn new_lease_code_non_admin_rejected() {
    let mut deps = deps();
    instantiate(
        deps.as_mut(),
        testing::mock_env(),
        sender(super::CREATOR),
        instantiate_msg(),
    )
    .unwrap();

    let err = execute(
        deps.as_mut(),
        testing::mock_env(),
        sender(NON_ADMIN),
        ExecuteMsg::NewLeaseCode {
            lease_code: Code::unchecked(LEASE_CODE_ID + 1),
        },
    )
    .unwrap_err();
    assert!(matches!(err, Error::Unauthorized(_)), "got {err:?}");

    let config = query_config(deps.as_ref());
    assert_eq!(Uint64::from(LEASE_CODE_ID), config.lease_code_id);
}

fn open_channel_by(deps: DepsMut<'_>, who: &str, ics20_channel: &str) -> Result<CwResponse, Error> {
    execute(
        deps,
        testing::mock_env(),
        sender(who),
        ExecuteMsg::OpenChannel {
            ics20_channel_remote: ics20_channel_remote(ics20_channel),
        },
    )
}

fn cancel_by(deps: DepsMut<'_>, who: &str) -> Result<CwResponse, Error> {
    execute(
        deps,
        testing::mock_env(),
        sender(who),
        ExecuteMsg::CancelChannelProposal(),
    )
}
