use serde::{Deserialize, Serialize};

use sdk::{cosmwasm_std::Storage, cw_storage_plus::Item};

use crate::error::{Error, Result};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChannelState {
    Open,
    Closing,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Channel {
    local_channel_id: String,
    counterparty_channel_id: String,
    counterparty_port_id: String,
    version: String,
    state: ChannelState,
}

impl Channel {
    const STORAGE: Item<Self> = Item::new("channel");

    pub fn new_open(
        local_channel_id: String,
        counterparty_channel_id: String,
        counterparty_port_id: String,
        version: String,
    ) -> Self {
        Self {
            local_channel_id,
            counterparty_channel_id,
            counterparty_port_id,
            version,
            state: ChannelState::Open,
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

    pub const fn state(&self) -> ChannelState {
        self.state
    }

    pub fn local_channel_id(&self) -> &str {
        &self.local_channel_id
    }

    /// Transitions an `Open` channel to `Closing`.
    /// Returns `ChannelNotOperational` if the channel is already `Closing`.
    pub fn into_closing(self) -> Result<Self> {
        match self.state {
            ChannelState::Open => Ok(Self {
                state: ChannelState::Closing,
                ..self
            }),
            ChannelState::Closing => Err(Error::ChannelNotOperational),
        }
    }

    /// Guard for outbound packet emission: accept only `Open`.
    pub fn usable_or_err(&self) -> Result<()> {
        match self.state {
            ChannelState::Open => Ok(()),
            ChannelState::Closing => Err(Error::ChannelNotOperational),
        }
    }

    pub(super) fn into_parts(self) -> (String, String, String, String, ChannelState) {
        (
            self.local_channel_id,
            self.counterparty_channel_id,
            self.counterparty_port_id,
            self.version,
            self.state,
        )
    }
}

#[cfg(test)]
mod test {
    use sdk::cosmwasm_std::testing::MockStorage;

    use crate::error::Error;

    use super::{Channel, ChannelState};

    const LOCAL_CHANNEL_ID: &str = "channel-7";
    const COUNTERPARTY_CHANNEL_ID: &str = "channel-42";
    const COUNTERPARTY_PORT_ID: &str = "nls-remote-lease.osmosis";
    const VERSION: &str = "nls-remote-lease.v1";

    #[test]
    fn may_load_empty() {
        let store = MockStorage::new();
        assert_eq!(None, Channel::may_load(&store).unwrap());
    }

    #[test]
    fn store_load_open() {
        let mut store = MockStorage::new();
        let channel = open_channel();
        channel.store(&mut store).unwrap();
        assert_eq!(Some(channel), Channel::may_load(&store).unwrap());
    }

    #[test]
    fn into_closing_from_open() {
        let channel = open_channel();
        let closing = channel.clone().into_closing().unwrap();
        assert_eq!(ChannelState::Open, channel.state());
        assert_eq!(ChannelState::Closing, closing.state());
        assert_eq!(channel.local_channel_id(), closing.local_channel_id());
    }

    #[test]
    fn into_closing_from_closing_errors() {
        let closing = open_channel().into_closing().unwrap();
        let err = closing.into_closing().unwrap_err();
        assert!(matches!(err, Error::ChannelNotOperational), "got {err:?}");
    }

    #[test]
    fn usable_or_err_open() {
        open_channel().usable_or_err().unwrap();
    }

    #[test]
    fn usable_or_err_closing() {
        let err = open_channel()
            .into_closing()
            .unwrap()
            .usable_or_err()
            .unwrap_err();
        assert!(matches!(err, Error::ChannelNotOperational), "got {err:?}");
    }

    #[test]
    fn clear_removes() {
        let mut store = MockStorage::new();
        open_channel().store(&mut store).unwrap();
        assert!(Channel::may_load(&store).unwrap().is_some());
        Channel::clear(&mut store);
        assert!(Channel::may_load(&store).unwrap().is_none());
    }

    fn open_channel() -> Channel {
        Channel::new_open(
            LOCAL_CHANNEL_ID.into(),
            COUNTERPARTY_CHANNEL_ID.into(),
            COUNTERPARTY_PORT_ID.into(),
            VERSION.into(),
        )
    }
}
