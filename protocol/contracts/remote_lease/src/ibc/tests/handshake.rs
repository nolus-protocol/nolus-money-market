use sdk::cosmwasm_std::{
    DepsMut, IbcChannel, IbcChannelCloseMsg, IbcChannelConnectMsg, IbcChannelOpenMsg, IbcOrder,
    testing,
};

use crate::{
    error::Error,
    state::{Channel, ChannelState},
};

use super::{
    CONNECTION_ID, COUNTERPARTY_CHANNEL_ID, COUNTERPARTY_PORT_ID, LOCAL_CHANNEL_ID,
    OTHER_PROPOSED_VERSION, PROPOSED_CHANNEL, PROPOSED_VERSION, VERSION, WRONG_CONNECTION_ID,
    WRONG_COUNTERPARTY_PORT_ID, WRONG_VERSION, channel, channel_on, deps_with_config,
    deps_with_init_accepted, deps_with_proposal,
};

use crate::ibc::{ibc_channel_close, ibc_channel_connect, ibc_channel_open};

const OTHER_LOCAL_CHANNEL_ID: &str = "channel-9";

// ---------------------------------------------------------------------------
// OpenInit — consumes `Proposed`, records the chain-assigned local channel id
// ---------------------------------------------------------------------------

#[test]
fn open_init_valid_consumes_the_proposal() {
    let mut deps = deps_with_proposal();
    let response = ibc_channel_open(
        deps.as_mut(),
        testing::mock_env(),
        open_init_msg(proposed_channel()),
    )
    .unwrap();
    assert!(response.is_none());

    assert_eq!(
        Channel::InitAccepted {
            ics20_channel_remote: proposed_pairing(),
            local_channel_id: LOCAL_CHANNEL_ID.into(),
        },
        Channel::may_load(&deps.storage).unwrap().unwrap(),
    );
}

// The M-1 race fix. wasmd dispatches our `MsgChannelOpenInit` in the same
// transaction that emitted it, so the first callback consumes the proposal
// atomically; a second one — an attacker racing his own INIT onto our port —
// finds nothing left.
#[test]
fn open_init_twice_rejected() {
    let mut deps = deps_with_proposal();
    ibc_channel_open(
        deps.as_mut(),
        testing::mock_env(),
        open_init_msg(proposed_channel()),
    )
    .unwrap();

    let err = ibc_channel_open(
        deps.as_mut(),
        testing::mock_env(),
        open_init_msg(proposed_channel()),
    )
    .unwrap_err();
    assert!(matches!(err, Error::UnsolicitedChannelOpen), "got {err:?}");

    // The first INIT's record is untouched by the rejected second one.
    assert_eq!(
        LOCAL_CHANNEL_ID,
        Channel::may_load(&deps.storage)
            .unwrap()
            .unwrap()
            .local_channel_id()
            .unwrap(),
    );
}

#[test]
fn open_init_without_proposal_rejected() {
    let mut deps = deps_with_config();
    let err = ibc_channel_open(
        deps.as_mut(),
        testing::mock_env(),
        open_init_msg(proposed_channel()),
    )
    .unwrap_err();
    assert!(matches!(err, Error::UnsolicitedChannelOpen), "got {err:?}");

    assert!(Channel::may_load(&deps.storage).unwrap().is_none());
}

#[test]
fn open_init_wrong_counterparty_port_rejected() {
    assert_open_init_rejected(
        channel(
            IbcOrder::Unordered,
            PROPOSED_VERSION,
            CONNECTION_ID,
            WRONG_COUNTERPARTY_PORT_ID,
        ),
        |err| matches!(err, Error::InvalidCounterpartyPort { .. }),
    );
}

#[test]
fn open_init_wrong_version_rejected() {
    assert_open_init_rejected(
        channel(
            IbcOrder::Unordered,
            WRONG_VERSION,
            CONNECTION_ID,
            COUNTERPARTY_PORT_ID,
        ),
        |err| matches!(err, Error::InvalidChannelVersion { .. }),
    );
}

// The handshake proposes the suffixed version; the bare one belongs on packets
// only. Accepting it here would open a channel the counterparty never paired a
// transfer channel with.
#[test]
fn open_init_bare_version_rejected() {
    assert_open_init_rejected(
        channel(
            IbcOrder::Unordered,
            VERSION,
            CONNECTION_ID,
            COUNTERPARTY_PORT_ID,
        ),
        |err| {
            matches!(
                err,
                Error::InvalidChannelVersion { expected, actual }
                    if expected == PROPOSED_VERSION && actual == VERSION,
            )
        },
    );
}

#[test]
fn open_init_ordered_rejected() {
    assert_open_init_rejected(
        channel(
            IbcOrder::Ordered,
            PROPOSED_VERSION,
            CONNECTION_ID,
            COUNTERPARTY_PORT_ID,
        ),
        |err| matches!(err, Error::InvalidChannelOrdering),
    );
}

#[test]
fn open_init_wrong_connection_rejected() {
    assert_open_init_rejected(
        channel(
            IbcOrder::Unordered,
            PROPOSED_VERSION,
            WRONG_CONNECTION_ID,
            COUNTERPARTY_PORT_ID,
        ),
        |err| matches!(err, Error::InvalidConnectionId { .. }),
    );
}

#[test]
fn open_try_rejected() {
    let mut deps = deps_with_proposal();
    let err = ibc_channel_open(
        deps.as_mut(),
        testing::mock_env(),
        IbcChannelOpenMsg::OpenTry {
            channel: proposed_channel(),
            counterparty_version: PROPOSED_VERSION.into(),
        },
    )
    .unwrap_err();
    assert!(
        matches!(err, Error::UnsupportedCounterpartyOpen),
        "got {err:?}"
    );
}

// ---------------------------------------------------------------------------
// OpenAck — requires `InitAccepted`, pins the local channel id
// ---------------------------------------------------------------------------

#[test]
fn connect_open_ack_establishes_channel() {
    let mut deps = deps_with_init_accepted();
    ibc_channel_connect(
        deps.as_mut(),
        testing::mock_env(),
        open_ack(PROPOSED_VERSION),
    )
    .unwrap();

    assert_eq!(
        Channel::Established {
            local_channel_id: LOCAL_CHANNEL_ID.into(),
            counterparty_channel_id: COUNTERPARTY_CHANNEL_ID.into(),
            counterparty_port_id: COUNTERPARTY_PORT_ID.into(),
            ics20_channel_remote: proposed_pairing(),
            state: ChannelState::Open,
        },
        Channel::may_load(&deps.storage).unwrap().unwrap(),
    );
}

// An ack whose `OpenInit` never ran: the proposal is still unconsumed, so this
// ack belongs to no handshake this controller completed.
#[test]
fn connect_open_ack_before_init_rejected() {
    let mut deps = deps_with_proposal();
    let err = ibc_channel_connect(
        deps.as_mut(),
        testing::mock_env(),
        open_ack(PROPOSED_VERSION),
    )
    .unwrap_err();
    assert!(matches!(err, Error::UnsolicitedChannelOpen), "got {err:?}");
}

#[test]
fn connect_without_any_record_rejected() {
    let mut deps = deps_with_config();
    let err = ibc_channel_connect(
        deps.as_mut(),
        testing::mock_env(),
        open_ack(PROPOSED_VERSION),
    )
    .unwrap_err();
    assert!(matches!(err, Error::UnsolicitedChannelOpen), "got {err:?}");

    assert!(Channel::may_load(&deps.storage).unwrap().is_none());
}

// The ack must belong to the channel our own INIT opened, not to some other
// channel that reached ack first.
#[test]
fn connect_open_ack_local_channel_id_mismatch_rejected() {
    let mut deps = deps_with_init_accepted();
    let err = ibc_channel_connect(
        deps.as_mut(),
        testing::mock_env(),
        IbcChannelConnectMsg::OpenAck {
            channel: channel_on(
                OTHER_LOCAL_CHANNEL_ID,
                IbcOrder::Unordered,
                PROPOSED_VERSION,
                CONNECTION_ID,
                COUNTERPARTY_PORT_ID,
            ),
            counterparty_version: PROPOSED_VERSION.into(),
        },
    )
    .unwrap_err();
    assert!(matches!(
        err,
        Error::LocalChannelIdMismatch { ref expected, ref actual }
            if expected == LOCAL_CHANNEL_ID && actual == OTHER_LOCAL_CHANNEL_ID,
    ));
}

// The counterparty echoes the version it accepted verbatim, so a bare echo means
// it never bound the paired transfer channel.
#[test]
fn connect_open_ack_bare_counterparty_version_rejected() {
    assert_open_ack_rejected(VERSION, |err| {
        matches!(
            err,
            Error::InvalidCounterpartyVersion { expected, actual }
                if expected == PROPOSED_VERSION && actual == VERSION,
        )
    });
}

// A counterparty that bound a transfer channel of its own choosing rather than
// the one proposed.
#[test]
fn connect_open_ack_mismatched_counterparty_suffix_rejected() {
    assert_open_ack_rejected(OTHER_PROPOSED_VERSION, |err| {
        matches!(
            err,
            Error::InvalidCounterpartyVersion { expected, actual }
                if expected == PROPOSED_VERSION && actual == OTHER_PROPOSED_VERSION,
        )
    });
}

// A hostile counterparty cannot inflate storage or events through the echo: the
// retained value is capped by our own grammar, not by its input.
#[test]
fn connect_open_ack_oversized_counterparty_version_bounded() {
    const CHANNEL_VERSION_MAX_BYTES: usize = 42;

    let oversized = "x".repeat(CHANNEL_VERSION_MAX_BYTES * 4);
    let mut deps = deps_with_init_accepted();
    let err =
        ibc_channel_connect(deps.as_mut(), testing::mock_env(), open_ack(&oversized)).unwrap_err();
    match err {
        Error::InvalidCounterpartyVersion { actual, .. } => {
            assert_eq!(CHANNEL_VERSION_MAX_BYTES, actual.len());
        }
        other => panic!("expected InvalidCounterpartyVersion, got {other:?}"),
    }
}

#[test]
fn connect_open_ack_when_established_rejected() {
    let mut deps = deps_with_init_accepted();
    ibc_channel_connect(
        deps.as_mut(),
        testing::mock_env(),
        open_ack(PROPOSED_VERSION),
    )
    .unwrap();

    let err = ibc_channel_connect(
        deps.as_mut(),
        testing::mock_env(),
        open_ack(PROPOSED_VERSION),
    )
    .unwrap_err();
    assert!(matches!(err, Error::ChannelAlreadyExists), "got {err:?}");
}

#[test]
fn connect_rejects_invalid_handshake_params() {
    assert_open_ack_channel_rejected(
        channel(
            IbcOrder::Unordered,
            WRONG_VERSION,
            CONNECTION_ID,
            COUNTERPARTY_PORT_ID,
        ),
        PROPOSED_VERSION,
        |err| matches!(err, Error::InvalidChannelVersion { .. }),
    );
}

// `OpenConfirm` is the callback of an `OpenTry` this contract rejects, and it
// carries no counterparty version to prove the pairing — fail closed.
#[test]
fn connect_open_confirm_rejected() {
    let mut deps = deps_with_init_accepted();
    let err = ibc_channel_connect(
        deps.as_mut(),
        testing::mock_env(),
        IbcChannelConnectMsg::OpenConfirm {
            channel: proposed_channel(),
        },
    )
    .unwrap_err();
    assert!(
        matches!(err, Error::UnsupportedCounterpartyOpen),
        "got {err:?}"
    );

    assert!(matches!(
        Channel::may_load(&deps.storage).unwrap().unwrap(),
        Channel::InitAccepted { .. },
    ));
}

// ---------------------------------------------------------------------------
// Close — over an `Established` channel only
// ---------------------------------------------------------------------------

// `CloseInit` is the terminal edge of a controller-initiated close: ibc-go
// closes the local end within the same `MsgChannelCloseInit`, so the record
// is cleared here — no later callback arrives on the initiating side.
#[test]
fn close_init_when_closing_clears_channel() {
    let mut deps = deps_with_config();
    persist_closing_channel(deps.as_mut());

    ibc_channel_close(
        deps.as_mut(),
        testing::mock_env(),
        IbcChannelCloseMsg::CloseInit {
            channel: proposed_channel(),
        },
    )
    .unwrap();

    assert!(Channel::may_load(&deps.storage).unwrap().is_none());
}

#[test]
fn close_init_when_open_rejected() {
    let mut deps = deps_with_config();
    persist_established_channel(deps.as_mut());

    assert_close_init_rejected(deps.as_mut());
}

#[test]
fn close_init_when_no_channel_rejected() {
    let mut deps = deps_with_config();
    assert_close_init_rejected(deps.as_mut());
}

// A close for a handshake that never completed is as unsolicited as one for an
// open channel we did not ask to close.
#[test]
fn close_init_while_handshake_in_flight_rejected() {
    let mut deps = deps_with_proposal();
    assert_close_init_rejected(deps.as_mut());

    let mut deps = deps_with_init_accepted();
    assert_close_init_rejected(deps.as_mut());
}

// `CloseConfirm` runs on the passive side of a close handshake, and the
// counterparty never initiates one — unreachable here, so it fails closed.
#[test]
fn close_confirm_rejected() {
    let mut deps = deps_with_config();
    persist_closing_channel(deps.as_mut());

    let err = ibc_channel_close(
        deps.as_mut(),
        testing::mock_env(),
        IbcChannelCloseMsg::CloseConfirm {
            channel: proposed_channel(),
        },
    )
    .unwrap_err();
    assert!(matches!(err, Error::UnsolicitedChannelClose), "got {err:?}");

    assert!(Channel::may_load(&deps.storage).unwrap().is_some());
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

fn proposed_channel() -> IbcChannel {
    channel(
        IbcOrder::Unordered,
        PROPOSED_VERSION,
        CONNECTION_ID,
        COUNTERPARTY_PORT_ID,
    )
}

fn open_ack(counterparty_version: &str) -> IbcChannelConnectMsg {
    IbcChannelConnectMsg::OpenAck {
        channel: proposed_channel(),
        counterparty_version: counterparty_version.into(),
    }
}

fn open_init_msg(channel: IbcChannel) -> IbcChannelOpenMsg {
    IbcChannelOpenMsg::OpenInit { channel }
}

fn assert_open_init_rejected<F>(channel: IbcChannel, expected: F)
where
    F: FnOnce(Error) -> bool,
{
    let mut deps = deps_with_proposal();
    let err =
        ibc_channel_open(deps.as_mut(), testing::mock_env(), open_init_msg(channel)).unwrap_err();
    assert!(expected(err), "unexpected rejection");

    // A rejected INIT must leave the proposal intact and re-usable.
    assert!(matches!(
        Channel::may_load(&deps.storage).unwrap().unwrap(),
        Channel::Proposed { .. },
    ));
}

fn assert_open_ack_rejected<F>(counterparty_version: &str, expected: F)
where
    F: FnOnce(Error) -> bool,
{
    assert_open_ack_channel_rejected(proposed_channel(), counterparty_version, expected);
}

fn assert_open_ack_channel_rejected<F>(channel: IbcChannel, counterparty_version: &str, expected: F)
where
    F: FnOnce(Error) -> bool,
{
    let mut deps = deps_with_init_accepted();
    let err = ibc_channel_connect(
        deps.as_mut(),
        testing::mock_env(),
        IbcChannelConnectMsg::OpenAck {
            channel,
            counterparty_version: counterparty_version.into(),
        },
    )
    .unwrap_err();
    assert!(expected(err), "unexpected rejection");

    // A rejected ack must not open the channel, and must leave the handshake
    // recoverable rather than half-torn-down.
    assert!(matches!(
        Channel::may_load(&deps.storage).unwrap().unwrap(),
        Channel::InitAccepted { .. },
    ));
}

fn assert_close_init_rejected(deps: DepsMut<'_>) {
    let err = ibc_channel_close(
        deps,
        testing::mock_env(),
        IbcChannelCloseMsg::CloseInit {
            channel: proposed_channel(),
        },
    )
    .unwrap_err();
    assert!(matches!(err, Error::UnsolicitedChannelClose), "got {err:?}");
}

fn persist_established_channel(deps: DepsMut<'_>) {
    established().store(deps.storage).unwrap();
}

fn persist_closing_channel(deps: DepsMut<'_>) {
    established()
        .into_closing()
        .unwrap()
        .store(deps.storage)
        .unwrap();
}

fn established() -> Channel {
    Channel::Established {
        local_channel_id: LOCAL_CHANNEL_ID.into(),
        counterparty_channel_id: COUNTERPARTY_CHANNEL_ID.into(),
        counterparty_port_id: COUNTERPARTY_PORT_ID.into(),
        ics20_channel_remote: proposed_pairing(),
        state: ChannelState::Open,
    }
}

fn proposed_pairing() -> remote_lease::Ics20ChannelId {
    PROPOSED_CHANNEL.parse().expect("a canonical channel id")
}
