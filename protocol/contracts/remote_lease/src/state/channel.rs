use serde::{Deserialize, Serialize};

use remote_lease::Ics20ChannelId;
use sdk::{cosmwasm_std::Storage, cw_storage_plus::Item};

use crate::error::{Error, Result};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChannelState {
    Open,
    Closing,
}

/// The controller's single channel record, as a state machine.
///
/// One item, one key: the handshake phases and the live channel are mutually
/// exclusive, so representing them as separate storage items would allow
/// combinations that cannot occur — a proposal beside an open channel, or an
/// ack for a handshake whose `OpenInit` never ran.
///
/// ```text
/// (absent) --OpenChannel--> Proposed --OnChanOpenInit--> InitAccepted
///                                                             |
///                                                          OpenAck
///                                                             v
///                              Established{Closing} <--   Established{Open}
///                                       |            CloseChannel
///                                 OnChanCloseInit
///                                       v
///                                   (absent)
/// ```
///
/// `InitAccepted` is what makes the `OpenInit` callback single-use: a second
/// `OpenInit` — whoever submits it — finds no proposal and is rejected. It
/// also records the chain-assigned local channel id, which `OpenAck` then
/// pins, so an ack for a different channel cannot be mistaken for ours.
// Single-use holds because wasmd dispatches the `MsgChannelOpenInit` this
// contract emits within the same transaction, so the callback consumes
// `Proposed` atomically with the emission.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Channel {
    Proposed {
        /// The transfer channel proposed for pairing; the derived handshake
        /// version is obtained through [`Channel::version`].
        // Stored as the typed id rather than the composed version string: the
        // id is canonical by construction and `channel_version()` is a pure
        // function of it, so recomposition is byte-stable. Two bytes of state
        // carrying the invariant beat 42 carrying the same one.
        ics20_channel_remote: Ics20ChannelId,
    },
    InitAccepted {
        ics20_channel_remote: Ics20ChannelId,
        /// The chain-assigned channel identifier.
        // Deliberately an opaque `String` — it is only stored, compared, and
        // echoed. `Ics20ChannelId` would be the *wrong* type for it and for
        // `counterparty_channel_id`: its `u16` bound is a Solana seed-level
        // constraint on the transfer channel, whereas ibc-go channel ordinals
        // are `u64` and a busy chain can exceed 65535.
        local_channel_id: String,
    },
    Established {
        local_channel_id: String,
        counterparty_channel_id: String,
        counterparty_port_id: String,
        ics20_channel_remote: Ics20ChannelId,
        state: ChannelState,
    },
}

impl Channel {
    const STORAGE: Item<Self> = Item::new("channel");

    pub const fn proposed(ics20_channel_remote: Ics20ChannelId) -> Self {
        Self::Proposed {
            ics20_channel_remote,
        }
    }

    pub fn may_load(storage: &dyn Storage) -> Result<Option<Self>> {
        Self::STORAGE.may_load(storage).map_err(Into::into)
    }

    pub fn store(&self, storage: &mut dyn Storage) -> Result<()> {
        Self::STORAGE.save(storage, self).map_err(Into::into)
    }

    pub fn clear(storage: &mut dyn Storage) {
        Self::STORAGE.remove(storage)
    }

    /// The transfer channel this record proposes pairing with. Fixed at
    /// proposal and carried through every transition unchanged.
    pub const fn ics20_channel_remote(&self) -> Ics20ChannelId {
        match self {
            Self::Proposed {
                ics20_channel_remote,
            }
            | Self::InitAccepted {
                ics20_channel_remote,
                ..
            }
            | Self::Established {
                ics20_channel_remote,
                ..
            } => *ics20_channel_remote,
        }
    }

    /// The handshake version proposed to the counterparty, and — once open —
    /// the version the channel carries.
    pub fn version(&self) -> String {
        self.ics20_channel_remote().channel_version()
    }

    /// The chain-assigned local channel id, once the handshake has progressed
    /// far enough for one to exist.
    pub fn local_channel_id(&self) -> Result<&str> {
        match self {
            Self::Proposed { .. } => Err(Error::ChannelNotEstablished),
            Self::InitAccepted {
                local_channel_id, ..
            }
            | Self::Established {
                local_channel_id, ..
            } => Ok(local_channel_id),
        }
    }

    /// `Proposed` → `InitAccepted`, consuming the proposal.
    ///
    /// Every other state means this `OpenInit` was not the one this controller
    /// asked for — including a second callback against a proposal already
    /// consumed.
    pub fn into_init_accepted(self, local_channel_id: String) -> Result<Self> {
        match self {
            Self::Proposed {
                ics20_channel_remote,
            } => Ok(Self::InitAccepted {
                ics20_channel_remote,
                local_channel_id,
            }),
            Self::InitAccepted { .. } | Self::Established { .. } => {
                Err(Error::UnsolicitedChannelOpen)
            }
        }
    }

    /// `InitAccepted` → `Established { Open }`.
    ///
    /// Pins the chain-assigned local channel id against the one recorded at
    /// `OpenInit`, so an ack belonging to some other channel cannot complete
    /// this handshake.
    pub fn into_established(
        self,
        local_channel_id: String,
        counterparty_channel_id: String,
        counterparty_port_id: String,
    ) -> Result<Self> {
        match self {
            Self::InitAccepted {
                ics20_channel_remote,
                local_channel_id: expected,
            } => {
                if expected == local_channel_id {
                    Ok(Self::Established {
                        local_channel_id: expected,
                        counterparty_channel_id,
                        counterparty_port_id,
                        ics20_channel_remote,
                        state: ChannelState::Open,
                    })
                } else {
                    Err(Error::LocalChannelIdMismatch {
                        expected,
                        actual: local_channel_id,
                    })
                }
            }
            Self::Proposed { .. } => Err(Error::UnsolicitedChannelOpen),
            Self::Established { .. } => Err(Error::ChannelAlreadyExists),
        }
    }

    /// Transitions an `Open` channel to `Closing`.
    pub fn into_closing(self) -> Result<Self> {
        match self {
            Self::Established {
                local_channel_id,
                counterparty_channel_id,
                counterparty_port_id,
                ics20_channel_remote,
                state: ChannelState::Open,
            } => Ok(Self::Established {
                local_channel_id,
                counterparty_channel_id,
                counterparty_port_id,
                ics20_channel_remote,
                state: ChannelState::Closing,
            }),
            Self::Established {
                state: ChannelState::Closing,
                ..
            } => Err(Error::ChannelNotOperational),
            Self::Proposed { .. } | Self::InitAccepted { .. } => Err(Error::ChannelNotEstablished),
        }
    }

    /// The local channel id, gated on the channel being open and live — the
    /// only way to obtain an id to emit a packet on, so a call site cannot
    /// take the id while forgetting the guard.
    pub fn usable_channel_id(&self) -> Result<&str> {
        self.usable_or_err().and_then(|()| self.local_channel_id())
    }

    /// Guard for outbound packet emission: accept only an open, live channel.
    fn usable_or_err(&self) -> Result<()> {
        match self {
            Self::Established {
                state: ChannelState::Open,
                ..
            } => Ok(()),
            Self::Established {
                state: ChannelState::Closing,
                ..
            } => Err(Error::ChannelNotOperational),
            Self::Proposed { .. } | Self::InitAccepted { .. } => Err(Error::ChannelNotEstablished),
        }
    }

    /// Guard for the counterparty's `CloseInit`: accept only a close this
    /// controller already asked for.
    pub fn close_init_or_err(&self) -> Result<()> {
        match self {
            Self::Established {
                state: ChannelState::Closing,
                ..
            } => Ok(()),
            Self::Established {
                state: ChannelState::Open,
                ..
            }
            | Self::Proposed { .. }
            | Self::InitAccepted { .. } => Err(Error::UnsolicitedChannelClose),
        }
    }

    /// Guard for the operator's cancel: only a handshake still in flight can be
    /// abandoned.
    pub fn cancellable_or_err(&self) -> Result<()> {
        match self {
            Self::Proposed { .. } | Self::InitAccepted { .. } => Ok(()),
            Self::Established { .. } => Err(Error::ChannelAlreadyExists),
        }
    }

    /// Why a fresh proposal cannot be recorded over this state.
    ///
    /// An in-flight handshake is never replaced silently — the operator
    /// abandons it explicitly through `CancelChannelProposal` — so this is
    /// total: no state admits a new proposal.
    pub fn new_proposal_rejection(&self) -> Error {
        match self {
            Self::Proposed { .. } | Self::InitAccepted { .. } => Error::ProposalPending,
            Self::Established { .. } => Error::ChannelAlreadyExists,
        }
    }
}

#[cfg(test)]
mod test {
    use sdk::cosmwasm_std::testing::MockStorage;

    use crate::error::Error;

    use super::{Channel, ChannelState};

    const ICS20_CHANNEL_REMOTE: &str = "channel-5";
    const VERSION: &str = "nls-remote-lease.v1+transfer=channel-5";
    const LOCAL_CHANNEL_ID: &str = "channel-7";
    const OTHER_LOCAL_CHANNEL_ID: &str = "channel-8";
    const COUNTERPARTY_CHANNEL_ID: &str = "channel-42";
    const COUNTERPARTY_PORT_ID: &str = "nls-remote-lease.osmosis";

    #[test]
    fn may_load_empty() {
        let store = MockStorage::new();
        assert_eq!(None, Channel::may_load(&store).unwrap());
    }

    #[test]
    fn proposed_composes_the_version() {
        assert_eq!(VERSION, proposed().version());
        assert_eq!(
            ICS20_CHANNEL_REMOTE,
            proposed().ics20_channel_remote().to_string(),
        );
    }

    #[test]
    fn store_load_round_trips_each_phase() {
        for channel in [proposed(), init_accepted(), established()] {
            let mut store = MockStorage::new();
            channel.store(&mut store).unwrap();
            assert_eq!(Some(channel), Channel::may_load(&store).unwrap());
        }
    }

    #[test]
    fn clear_removes() {
        let mut store = MockStorage::new();
        proposed().store(&mut store).unwrap();
        assert!(Channel::may_load(&store).unwrap().is_some());
        Channel::clear(&mut store);
        assert!(Channel::may_load(&store).unwrap().is_none());
    }

    // The pairing is fixed at proposal and every later phase recomposes the
    // same bytes — that is what the handshake checks compare against.
    #[test]
    fn version_survives_every_transition() {
        for channel in [proposed(), init_accepted(), established(), closing()] {
            assert_eq!(VERSION, channel.version());
            assert_eq!(
                ICS20_CHANNEL_REMOTE,
                channel.ics20_channel_remote().to_string(),
            );
        }
    }

    #[test]
    fn local_channel_id_absent_while_only_proposed() {
        let err = proposed().local_channel_id().unwrap_err();
        assert!(matches!(err, Error::ChannelNotEstablished), "got {err:?}");
        assert_eq!(
            LOCAL_CHANNEL_ID,
            init_accepted().local_channel_id().unwrap()
        );
        assert_eq!(LOCAL_CHANNEL_ID, established().local_channel_id().unwrap());
    }

    #[test]
    fn into_init_accepted_consumes_the_proposal() {
        assert_eq!(init_accepted(), accept_init(proposed()).unwrap());
    }

    // The M-1 guard: a proposal is single-use, so a second `OpenInit` — the
    // attacker's, or a duplicate — has nothing left to consume.
    #[test]
    fn into_init_accepted_twice_rejected() {
        for channel in [init_accepted(), established()] {
            let err = accept_init(channel).unwrap_err();
            assert!(matches!(err, Error::UnsolicitedChannelOpen), "got {err:?}");
        }
    }

    #[test]
    fn into_established_from_init_accepted() {
        assert_eq!(
            established(),
            establish(init_accepted(), LOCAL_CHANNEL_ID).unwrap(),
        );
    }

    #[test]
    fn into_established_pins_the_local_channel_id() {
        let err = establish(init_accepted(), OTHER_LOCAL_CHANNEL_ID).unwrap_err();
        assert!(matches!(
            err,
            Error::LocalChannelIdMismatch { ref expected, ref actual }
                if expected == LOCAL_CHANNEL_ID && actual == OTHER_LOCAL_CHANNEL_ID,
        ));
    }

    // An ack whose `OpenInit` never ran must not open a channel.
    #[test]
    fn into_established_from_proposed_rejected() {
        let err = establish(proposed(), LOCAL_CHANNEL_ID).unwrap_err();
        assert!(matches!(err, Error::UnsolicitedChannelOpen), "got {err:?}");
    }

    #[test]
    fn into_established_when_already_established_rejected() {
        let err = establish(established(), LOCAL_CHANNEL_ID).unwrap_err();
        assert!(matches!(err, Error::ChannelAlreadyExists), "got {err:?}");
    }

    #[test]
    fn into_closing_from_open() {
        let closing = established().into_closing().unwrap();
        assert!(matches!(
            closing,
            Channel::Established {
                state: ChannelState::Closing,
                ..
            }
        ));
        assert_eq!(LOCAL_CHANNEL_ID, closing.local_channel_id().unwrap());
    }

    #[test]
    fn into_closing_from_closing_errors() {
        let err = closing().into_closing().unwrap_err();
        assert!(matches!(err, Error::ChannelNotOperational), "got {err:?}");
    }

    #[test]
    fn into_closing_before_established_errors() {
        for channel in [proposed(), init_accepted()] {
            let err = channel.into_closing().unwrap_err();
            assert!(matches!(err, Error::ChannelNotEstablished), "got {err:?}");
        }
    }

    #[test]
    fn usable_channel_id_only_when_established_and_open() {
        assert_eq!(LOCAL_CHANNEL_ID, established().usable_channel_id().unwrap());

        let err = closing().usable_channel_id().unwrap_err();
        assert!(matches!(err, Error::ChannelNotOperational), "got {err:?}");

        for channel in [proposed(), init_accepted()] {
            let err = channel.usable_channel_id().unwrap_err();
            assert!(matches!(err, Error::ChannelNotEstablished), "got {err:?}");
        }
    }

    #[test]
    fn close_init_only_when_closing() {
        closing().close_init_or_err().unwrap();

        for channel in [proposed(), init_accepted(), established()] {
            let err = channel.close_init_or_err().unwrap_err();
            assert!(matches!(err, Error::UnsolicitedChannelClose), "got {err:?}");
        }
    }

    #[test]
    fn cancellable_only_while_the_handshake_is_in_flight() {
        proposed().cancellable_or_err().unwrap();
        init_accepted().cancellable_or_err().unwrap();

        for channel in [established(), closing()] {
            let err = channel.cancellable_or_err().unwrap_err();
            assert!(matches!(err, Error::ChannelAlreadyExists), "got {err:?}");
        }
    }

    #[test]
    fn no_state_admits_a_second_proposal() {
        for channel in [proposed(), init_accepted()] {
            assert!(
                matches!(channel.new_proposal_rejection(), Error::ProposalPending),
                "an in-flight handshake must be cancelled, never replaced",
            );
        }
        for channel in [established(), closing()] {
            assert!(matches!(
                channel.new_proposal_rejection(),
                Error::ChannelAlreadyExists
            ));
        }
    }

    fn proposed() -> Channel {
        Channel::proposed(
            ICS20_CHANNEL_REMOTE
                .parse()
                .expect("a canonical channel id"),
        )
    }

    fn init_accepted() -> Channel {
        accept_init(proposed()).expect("a fresh proposal accepts its init")
    }

    fn established() -> Channel {
        establish(init_accepted(), LOCAL_CHANNEL_ID).expect("a matching ack establishes")
    }

    fn closing() -> Channel {
        established()
            .into_closing()
            .expect("an open channel starts closing")
    }

    fn accept_init(channel: Channel) -> Result<Channel, Error> {
        channel.into_init_accepted(LOCAL_CHANNEL_ID.into())
    }

    fn establish(channel: Channel, local_channel_id: &str) -> Result<Channel, Error> {
        channel.into_established(
            local_channel_id.into(),
            COUNTERPARTY_CHANNEL_ID.into(),
            COUNTERPARTY_PORT_ID.into(),
        )
    }
}
