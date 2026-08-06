use sdk::cosmwasm_std::testing;
use versioning::{ProtocolPackageRelease, package_name, package_version};

use crate::{
    api::{ChannelInfo, ChannelStateResponse, QueryMsg},
    contract::query,
};

use super::{
    CONTRACT_STORAGE_VERSION, ICS20_CHANNEL_REMOTE, LOCAL_CHANNEL_ID, PROPOSED_VERSION, deps,
    ics20_channel_remote, instantiate_default, query_channel, store_init_accepted,
    store_open_channel, store_proposal,
};

#[test]
fn query_protocol_package_release_returns_current() {
    let deps = deps();
    let raw = query(
        deps.as_ref(),
        testing::mock_env(),
        QueryMsg::ProtocolPackageRelease {},
    )
    .unwrap();
    let parsed: ProtocolPackageRelease = sdk::cosmwasm_std::from_json(raw).unwrap();
    let expected = ProtocolPackageRelease::current(
        package_name!(),
        package_version!(),
        CONTRACT_STORAGE_VERSION,
    );
    assert_eq!(
        sdk::cosmwasm_std::to_json_binary(&expected).unwrap(),
        sdk::cosmwasm_std::to_json_binary(&parsed).unwrap(),
    );
}

#[test]
fn query_channel_absent_before_any_proposal() {
    let mut deps = deps();
    instantiate_default(deps.as_mut());
    assert!(query_channel(deps.as_ref()).channel.is_none());
}

// The phase is the point of the query surface: an operator must be able to tell
// a proposal still waiting on its own INIT from one waiting on the counterparty.
#[test]
fn query_channel_reports_the_handshake_phase() {
    let mut deps = deps();
    instantiate_default(deps.as_mut());
    store_proposal(deps.as_mut());
    assert_eq!(
        ChannelInfo::Proposed {
            ics20_channel_remote: ics20_channel_remote(ICS20_CHANNEL_REMOTE),
            version: PROPOSED_VERSION.into(),
        },
        recorded(deps.as_ref()),
    );

    store_init_accepted(deps.as_mut());
    assert_eq!(
        ChannelInfo::InitAccepted {
            ics20_channel_remote: ics20_channel_remote(ICS20_CHANNEL_REMOTE),
            version: PROPOSED_VERSION.into(),
            local_channel_id: LOCAL_CHANNEL_ID.into(),
        },
        recorded(deps.as_ref()),
    );
}

// The typed pairing travels as the rendered id, beside the exact version bytes
// an operator diffs against the counterparty's logs.
#[test]
fn query_channel_exposes_the_pairing_on_the_wire() {
    let mut deps = deps();
    instantiate_default(deps.as_mut());
    store_proposal(deps.as_mut());

    let raw = query(deps.as_ref(), testing::mock_env(), QueryMsg::Channel()).unwrap();
    assert_eq!(
        format!(
            r#"{{"channel":{{"proposed":{{"ics20_channel_remote":"{ICS20_CHANNEL_REMOTE}","version":"{PROPOSED_VERSION}"}}}}}}"#
        ),
        String::from_utf8(raw.into()).expect("the response is UTF-8 JSON"),
    );
}

#[test]
fn query_channel_returns_open_state_when_channel_is_open() {
    let mut deps = deps();
    instantiate_default(deps.as_mut());
    store_open_channel(deps.as_mut());

    match recorded(deps.as_ref()) {
        ChannelInfo::Established {
            local_channel_id,
            ics20_channel_remote: pairing,
            version,
            state,
            ..
        } => {
            assert!(matches!(state, ChannelStateResponse::Open));
            assert_eq!(LOCAL_CHANNEL_ID, local_channel_id);
            assert_eq!(PROPOSED_VERSION, version);
            assert_eq!(ics20_channel_remote(ICS20_CHANNEL_REMOTE), pairing);
        }
        other => panic!("expected an established channel, got {other:?}"),
    }
}

fn recorded(deps: sdk::cosmwasm_std::Deps<'_>) -> ChannelInfo {
    query_channel(deps).channel.expect("a channel is recorded")
}
