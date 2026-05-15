use serde::{Deserialize, Serialize};

use sdk::{cosmwasm_std::Storage, cw_storage_plus::Item};

use crate::error::Result;

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

    pub fn may_load(storage: &dyn Storage) -> Result<Option<Self>> {
        Self::STORAGE.may_load(storage).map_err(Into::into)
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
    fn round_trip_open() {
        round_trip(ChannelState::Open);
    }

    #[test]
    fn round_trip_closing() {
        round_trip(ChannelState::Closing);
    }

    fn round_trip(state: ChannelState) {
        let mut store = MockStorage::new();
        let channel = Channel {
            local_channel_id: LOCAL_CHANNEL_ID.into(),
            counterparty_channel_id: COUNTERPARTY_CHANNEL_ID.into(),
            counterparty_port_id: COUNTERPARTY_PORT_ID.into(),
            version: VERSION.into(),
            state,
        };
        Channel::STORAGE.save(&mut store, &channel).unwrap();
        assert_eq!(Some(channel), Channel::may_load(&store).unwrap());
    }
}
