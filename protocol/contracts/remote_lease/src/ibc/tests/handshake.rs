use sdk::cosmwasm_std::{
    DepsMut, IbcChannel, IbcChannelCloseMsg, IbcChannelConnectMsg, IbcChannelOpenMsg, IbcOrder,
    testing,
};

use crate::{
    error::Error,
    state::{Channel, ChannelState},
};

use super::{
    BARE_VERSION, CONNECTION_ID, COUNTERPARTY_CHANNEL_ID, COUNTERPARTY_PORT_ID, LOCAL_CHANNEL_ID,
    VERSION, WRONG_CONNECTION_ID, WRONG_COUNTERPARTY_PORT_ID, WRONG_TRANSFER_VERSION,
    WRONG_VERSION, channel, deps_with_config,
};

use crate::ibc::{ibc_channel_close, ibc_channel_connect, ibc_channel_open};

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

// The pre-suffix grammar (bare protocol version, no `+transfer=` pairing) must
// no longer pass the handshake — the Solana responder requires the paired
// transfer channel since ibc-solray#322.
#[test]
fn open_init_bare_version_rejected() {
    let mut deps = deps_with_config();
    let err = ibc_channel_open(
        deps.as_mut(),
        testing::mock_env(),
        open_init_msg(channel(
            IbcOrder::Unordered,
            BARE_VERSION,
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
fn open_init_wrong_transfer_suffix_rejected() {
    let mut deps = deps_with_config();
    let err = ibc_channel_open(
        deps.as_mut(),
        testing::mock_env(),
        open_init_msg(channel(
            IbcOrder::Unordered,
            WRONG_TRANSFER_VERSION,
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
fn connect_open_ack_wrong_counterparty_version_rejected() {
    let mut deps = deps_with_config();
    let connect = IbcChannelConnectMsg::OpenAck {
        channel: channel(
            IbcOrder::Unordered,
            VERSION,
            CONNECTION_ID,
            COUNTERPARTY_PORT_ID,
        ),
        counterparty_version: WRONG_TRANSFER_VERSION.into(),
    };
    let err = ibc_channel_connect(deps.as_mut(), testing::mock_env(), connect).unwrap_err();
    assert!(
        matches!(err, Error::InvalidCounterpartyVersion { .. }),
        "got {err:?}"
    );
    assert!(Channel::may_load(&deps.storage).unwrap().is_none());
}

#[test]
fn connect_open_ack_oversized_counterparty_version_truncated_in_error() {
    const ECHO_CAP_CHARS: usize = 64;

    let mut deps = deps_with_config();
    let connect = IbcChannelConnectMsg::OpenAck {
        channel: channel(
            IbcOrder::Unordered,
            VERSION,
            CONNECTION_ID,
            COUNTERPARTY_PORT_ID,
        ),
        counterparty_version: "x".repeat(ECHO_CAP_CHARS * 4),
    };
    let err = ibc_channel_connect(deps.as_mut(), testing::mock_env(), connect).unwrap_err();
    match err {
        Error::InvalidCounterpartyVersion { actual, .. } => {
            assert_eq!(ECHO_CAP_CHARS, actual.chars().count());
        }
        other => panic!("expected InvalidCounterpartyVersion, got {other:?}"),
    }
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

// `connect_rejects_when_channel_exists` pre-loads the channel directly. This
// variant exercises the *sequence* — first OpenAck persists, second handshake
// callback (whether OpenAck or OpenConfirm) must reject. This is the test that
// answers the "simultaneous handshake" wording in ADR 0001 §3 by walking the
// sequential entry-point path; cw-multi-test is single-threaded so a real race
// cannot be exercised in-process.
#[test]
fn ibc_connect_rejects_when_channel_already_persisted() {
    let mut deps = deps_with_config();
    ibc_channel_connect(
        deps.as_mut(),
        testing::mock_env(),
        IbcChannelConnectMsg::OpenAck {
            channel: channel(
                IbcOrder::Unordered,
                VERSION,
                CONNECTION_ID,
                COUNTERPARTY_PORT_ID,
            ),
            counterparty_version: VERSION.into(),
        },
    )
    .unwrap();

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

fn open_init_msg(channel: IbcChannel) -> IbcChannelOpenMsg {
    IbcChannelOpenMsg::OpenInit { channel }
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
