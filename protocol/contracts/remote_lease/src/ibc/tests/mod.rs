mod handshake;
mod packets;

use sdk::{
    cosmwasm_std::{
        IbcChannel, IbcEndpoint, IbcOrder, MessageInfo, OwnedDeps,
        testing::{self, MockApi, MockQuerier, MockStorage},
    },
    testing as sdk_testing,
};

use crate::{api::InstantiateMsg, contract::instantiate, state::Channel};

const ADMIN: &str = "admin";
const CREATOR: &str = "creator";
const CONNECTION_ID: &str = "connection-3";
const WRONG_CONNECTION_ID: &str = "connection-9";
const DEX_LABEL: &str = "osmosis";
const LOCAL_PORT_ID: &str = "wasm.controller";
const LOCAL_CHANNEL_ID: &str = "channel-0";
const OTHER_LOCAL_CHANNEL_ID: &str = "channel-9";
const COUNTERPARTY_CHANNEL_ID: &str = "channel-77";
const COUNTERPARTY_PORT_ID: &str = "nls-remote-lease.osmosis";
const WRONG_COUNTERPARTY_PORT_ID: &str = "nls-remote-lease.evil";
// The bare protocol version — legal on a packet, never in the handshake.
const VERSION: &str = "nls-remote-lease.v1";
// Literal on purpose: composing these from the wire crate would let a grammar
// regression rewrite the expectations along with the code.
const PROPOSED_VERSION: &str = "nls-remote-lease.v1+transfer=channel-5";
const OTHER_PROPOSED_VERSION: &str = "nls-remote-lease.v1+transfer=channel-6";
const PROPOSED_CHANNEL: &str = "channel-5";
const WRONG_VERSION: &str = "nls-remote-lease.v2+transfer=channel-5";
const LEASE_CODE_ID: u64 = 17;

/// Config plus a recorded proposal — the state `OpenChannel` leaves behind, and
/// the only state the `OpenInit` callback accepts.
fn deps_with_proposal() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
    let mut deps = deps_with_config();
    proposal()
        .store(&mut deps.storage)
        .expect("storing a fresh proposal");
    deps
}

/// The state after the `OpenInit` callback has consumed the proposal — where an
/// `OpenAck` legitimately arrives.
fn deps_with_init_accepted() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
    let mut deps = deps_with_config();
    proposal()
        .into_init_accepted(LOCAL_CHANNEL_ID.into())
        .expect("a fresh proposal accepts its init")
        .store(&mut deps.storage)
        .expect("storing the accepted init");
    deps
}

/// The state a live channel is in — where packet callbacks legitimately arrive.
fn deps_with_established() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
    let mut deps = deps_with_config();
    proposal()
        .into_init_accepted(LOCAL_CHANNEL_ID.into())
        .and_then(|accepted| {
            accepted.into_established(
                LOCAL_CHANNEL_ID.into(),
                COUNTERPARTY_CHANNEL_ID.into(),
                COUNTERPARTY_PORT_ID.into(),
            )
        })
        .expect("the fixture walks a valid handshake")
        .store(&mut deps.storage)
        .expect("storing the established channel");
    deps
}

fn proposal() -> Channel {
    Channel::proposed(PROPOSED_CHANNEL.parse().expect("a canonical channel id"))
}

fn deps_with_config() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
    let mut deps = sdk_testing::mock_deps_with_contracts([]);
    instantiate(
        deps.as_mut(),
        testing::mock_env(),
        MessageInfo {
            sender: sdk_testing::user(CREATOR),
            funds: vec![],
        },
        InstantiateMsg {
            protocol_admin: sdk_testing::user(ADMIN).into_string(),
            connection_id: CONNECTION_ID.into(),
            dex_label: DEX_LABEL.into(),
            lease_code: LEASE_CODE_ID.into(),
        },
    )
    .unwrap();
    deps
}

fn channel(
    order: IbcOrder,
    version: &str,
    connection_id: &str,
    counterparty_port_id: &str,
) -> IbcChannel {
    channel_on(
        LOCAL_CHANNEL_ID,
        order,
        version,
        connection_id,
        counterparty_port_id,
    )
}

fn channel_on(
    local_channel_id: &str,
    order: IbcOrder,
    version: &str,
    connection_id: &str,
    counterparty_port_id: &str,
) -> IbcChannel {
    IbcChannel::new(
        IbcEndpoint {
            port_id: LOCAL_PORT_ID.into(),
            channel_id: local_channel_id.into(),
        },
        IbcEndpoint {
            port_id: counterparty_port_id.into(),
            channel_id: COUNTERPARTY_CHANNEL_ID.into(),
        },
        order,
        version,
        connection_id,
    )
}
