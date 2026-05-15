use sdk::{
    cosmwasm_std::{
        Binary, DepsMut, IbcChannel, IbcChannelCloseMsg, IbcChannelConnectMsg, IbcChannelOpenMsg,
        IbcEndpoint, IbcOrder, IbcPacket, IbcPacketReceiveMsg, IbcTimeout, MessageInfo, OwnedDeps,
        StdAck, Timestamp,
        testing::{self, MockApi, MockQuerier, MockStorage},
    },
    testing as sdk_testing,
};

use crate::{
    api::InstantiateMsg,
    contract::instantiate,
    error::Error,
    state::{Channel, ChannelState},
};

use super::{ibc_channel_close, ibc_channel_connect, ibc_channel_open, ibc_packet_receive};

const ADMIN: &str = "admin";
const CREATOR: &str = "creator";
const CONNECTION_ID: &str = "connection-3";
const WRONG_CONNECTION_ID: &str = "connection-9";
const DEX_LABEL: &str = "osmosis";
const LOCAL_PORT_ID: &str = "wasm.controller";
const LOCAL_CHANNEL_ID: &str = "channel-0";
const COUNTERPARTY_CHANNEL_ID: &str = "channel-77";
const COUNTERPARTY_PORT_ID: &str = "nls-remote-lease.osmosis";
const WRONG_COUNTERPARTY_PORT_ID: &str = "nls-remote-lease.evil";
const VERSION: &str = "nls-remote-lease.v1";
const WRONG_VERSION: &str = "nls-remote-lease.v2";
const LEASE_CODE_ID: u64 = 17;

#[test]
fn open_init_valid_succeeds() {
    let mut deps = deps_with_config();
    let response = ibc_channel_open(
        deps.as_mut(),
        testing::mock_env(),
        open_init_msg(channel(
            IbcOrder::Unordered,
            VERSION,
            CONNECTION_ID,
            COUNTERPARTY_PORT_ID,
        )),
    )
    .unwrap();
    assert!(response.is_none());

    assert!(Channel::may_load(&deps.storage).unwrap().is_none());
}

#[test]
fn open_init_wrong_counterparty_port_rejected() {
    let mut deps = deps_with_config();
    let err = ibc_channel_open(
        deps.as_mut(),
        testing::mock_env(),
        open_init_msg(channel(
            IbcOrder::Unordered,
            VERSION,
            CONNECTION_ID,
            WRONG_COUNTERPARTY_PORT_ID,
        )),
    )
    .unwrap_err();
    assert!(
        matches!(err, Error::InvalidCounterpartyPort { .. }),
        "got {err:?}"
    );
}

#[test]
fn open_init_wrong_version_rejected() {
    let mut deps = deps_with_config();
    let err = ibc_channel_open(
        deps.as_mut(),
        testing::mock_env(),
        open_init_msg(channel(
            IbcOrder::Unordered,
            WRONG_VERSION,
            CONNECTION_ID,
            COUNTERPARTY_PORT_ID,
        )),
    )
    .unwrap_err();
    assert!(
        matches!(err, Error::InvalidChannelVersion { .. }),
        "got {err:?}"
    );
}

#[test]
fn open_init_ordered_rejected() {
    let mut deps = deps_with_config();
    let err = ibc_channel_open(
        deps.as_mut(),
        testing::mock_env(),
        open_init_msg(channel(
            IbcOrder::Ordered,
            VERSION,
            CONNECTION_ID,
            COUNTERPARTY_PORT_ID,
        )),
    )
    .unwrap_err();
    assert!(matches!(err, Error::InvalidChannelOrdering), "got {err:?}");
}

#[test]
fn open_init_wrong_connection_rejected() {
    let mut deps = deps_with_config();
    let err = ibc_channel_open(
        deps.as_mut(),
        testing::mock_env(),
        open_init_msg(channel(
            IbcOrder::Unordered,
            VERSION,
            WRONG_CONNECTION_ID,
            COUNTERPARTY_PORT_ID,
        )),
    )
    .unwrap_err();
    assert!(
        matches!(err, Error::InvalidConnectionId { .. }),
        "got {err:?}"
    );
}

#[test]
fn open_try_rejected() {
    let mut deps = deps_with_config();
    let err = ibc_channel_open(
        deps.as_mut(),
        testing::mock_env(),
        IbcChannelOpenMsg::OpenTry {
            channel: channel(
                IbcOrder::Unordered,
                VERSION,
                CONNECTION_ID,
                COUNTERPARTY_PORT_ID,
            ),
            counterparty_version: VERSION.into(),
        },
    )
    .unwrap_err();
    assert!(
        matches!(err, Error::UnsupportedCounterpartyOpen),
        "got {err:?}"
    );
}

#[test]
fn connect_open_ack_persists_channel() {
    let mut deps = deps_with_config();
    let connect = IbcChannelConnectMsg::OpenAck {
        channel: channel(
            IbcOrder::Unordered,
            VERSION,
            CONNECTION_ID,
            COUNTERPARTY_PORT_ID,
        ),
        counterparty_version: VERSION.into(),
    };
    ibc_channel_connect(deps.as_mut(), testing::mock_env(), connect).unwrap();

    let stored = Channel::may_load(&deps.storage).unwrap().unwrap();
    assert_eq!(ChannelState::Open, stored.state());
    assert_eq!(LOCAL_CHANNEL_ID, stored.local_channel_id());
}

#[test]
fn connect_open_confirm_persists_channel() {
    let mut deps = deps_with_config();
    let connect = IbcChannelConnectMsg::OpenConfirm {
        channel: channel(
            IbcOrder::Unordered,
            VERSION,
            CONNECTION_ID,
            COUNTERPARTY_PORT_ID,
        ),
    };
    ibc_channel_connect(deps.as_mut(), testing::mock_env(), connect).unwrap();

    let stored = Channel::may_load(&deps.storage).unwrap().unwrap();
    assert_eq!(ChannelState::Open, stored.state());
}

#[test]
fn connect_rejects_when_channel_exists() {
    let mut deps = deps_with_config();
    persist_existing_open_channel(deps.as_mut());

    let err = ibc_channel_connect(
        deps.as_mut(),
        testing::mock_env(),
        IbcChannelConnectMsg::OpenConfirm {
            channel: channel(
                IbcOrder::Unordered,
                VERSION,
                CONNECTION_ID,
                COUNTERPARTY_PORT_ID,
            ),
        },
    )
    .unwrap_err();
    assert!(matches!(err, Error::ChannelAlreadyExists), "got {err:?}");
}

#[test]
fn connect_rejects_invalid_handshake_params() {
    let mut deps = deps_with_config();
    let err = ibc_channel_connect(
        deps.as_mut(),
        testing::mock_env(),
        IbcChannelConnectMsg::OpenConfirm {
            channel: channel(
                IbcOrder::Unordered,
                WRONG_VERSION,
                CONNECTION_ID,
                COUNTERPARTY_PORT_ID,
            ),
        },
    )
    .unwrap_err();
    assert!(
        matches!(err, Error::InvalidChannelVersion { .. }),
        "got {err:?}"
    );
}

#[test]
fn close_init_when_closing_accepted() {
    let mut deps = deps_with_config();
    persist_existing_closing_channel(deps.as_mut());

    ibc_channel_close(
        deps.as_mut(),
        testing::mock_env(),
        IbcChannelCloseMsg::CloseInit {
            channel: channel(
                IbcOrder::Unordered,
                VERSION,
                CONNECTION_ID,
                COUNTERPARTY_PORT_ID,
            ),
        },
    )
    .unwrap();

    assert!(Channel::may_load(&deps.storage).unwrap().is_some());
}

#[test]
fn close_init_when_open_rejected() {
    let mut deps = deps_with_config();
    persist_existing_open_channel(deps.as_mut());

    let err = ibc_channel_close(
        deps.as_mut(),
        testing::mock_env(),
        IbcChannelCloseMsg::CloseInit {
            channel: channel(
                IbcOrder::Unordered,
                VERSION,
                CONNECTION_ID,
                COUNTERPARTY_PORT_ID,
            ),
        },
    )
    .unwrap_err();
    assert!(matches!(err, Error::UnsolicitedChannelClose), "got {err:?}");
}

#[test]
fn close_init_when_no_channel_rejected() {
    let mut deps = deps_with_config();
    let err = ibc_channel_close(
        deps.as_mut(),
        testing::mock_env(),
        IbcChannelCloseMsg::CloseInit {
            channel: channel(
                IbcOrder::Unordered,
                VERSION,
                CONNECTION_ID,
                COUNTERPARTY_PORT_ID,
            ),
        },
    )
    .unwrap_err();
    assert!(matches!(err, Error::UnsolicitedChannelClose), "got {err:?}");
}

#[test]
fn close_confirm_clears_channel() {
    let mut deps = deps_with_config();
    persist_existing_closing_channel(deps.as_mut());

    ibc_channel_close(
        deps.as_mut(),
        testing::mock_env(),
        IbcChannelCloseMsg::CloseConfirm {
            channel: channel(
                IbcOrder::Unordered,
                VERSION,
                CONNECTION_ID,
                COUNTERPARTY_PORT_ID,
            ),
        },
    )
    .unwrap();

    assert!(Channel::may_load(&deps.storage).unwrap().is_none());
}

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

fn channel(
    order: IbcOrder,
    version: &str,
    connection_id: &str,
    counterparty_port_id: &str,
) -> IbcChannel {
    IbcChannel::new(
        IbcEndpoint {
            port_id: LOCAL_PORT_ID.into(),
            channel_id: LOCAL_CHANNEL_ID.into(),
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

fn open_init_msg(channel: IbcChannel) -> IbcChannelOpenMsg {
    IbcChannelOpenMsg::OpenInit { channel }
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

fn persist_existing_open_channel(deps: DepsMut<'_>) {
    Channel::new_open(
        LOCAL_CHANNEL_ID.into(),
        COUNTERPARTY_CHANNEL_ID.into(),
        COUNTERPARTY_PORT_ID.into(),
        VERSION.into(),
    )
    .store(deps.storage)
    .unwrap();
}

fn persist_existing_closing_channel(deps: DepsMut<'_>) {
    let closing = Channel::new_open(
        LOCAL_CHANNEL_ID.into(),
        COUNTERPARTY_CHANNEL_ID.into(),
        COUNTERPARTY_PORT_ID.into(),
        VERSION.into(),
    )
    .into_closing()
    .unwrap();
    closing.store(deps.storage).unwrap();
}
