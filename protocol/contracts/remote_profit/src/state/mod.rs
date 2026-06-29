use crate::api::{ChannelInfo, ChannelResponse, ChannelStateResponse, ConfigResponse};

pub(crate) use self::config::canonical_transfer_channel;
pub use self::{
    channel::{Channel, ChannelState},
    config::Config,
};

mod channel;
mod config;

impl From<Config> for ConfigResponse {
    fn from(cfg: Config) -> Self {
        let (connection_id, dex_label, transfer_channel, profit_code, profit_contract) =
            cfg.into_parts();
        Self::new(
            connection_id,
            dex_label,
            transfer_channel,
            profit_code,
            profit_contract.into_string(),
        )
    }
}

impl From<Option<Channel>> for ChannelResponse {
    fn from(channel: Option<Channel>) -> Self {
        Self {
            channel: channel.map(ChannelInfo::from),
        }
    }
}

impl From<Channel> for ChannelInfo {
    fn from(channel: Channel) -> Self {
        let (local_channel_id, counterparty_channel_id, counterparty_port_id, version, state) =
            channel.into_parts();
        Self {
            local_channel_id,
            counterparty_channel_id,
            counterparty_port_id,
            version,
            state: state.into(),
        }
    }
}

impl From<ChannelState> for ChannelStateResponse {
    fn from(state: ChannelState) -> Self {
        match state {
            ChannelState::Open => Self::Open,
            ChannelState::Closing => Self::Closing,
        }
    }
}
