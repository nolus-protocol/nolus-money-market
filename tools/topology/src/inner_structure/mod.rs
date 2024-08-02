use std::collections::BTreeMap;

use serde::Deserialize;

pub(crate) use self::{
    channel::Channel,
    currency::{Currency, Ibc as IbcCurrency, Native as NativeCurrency},
    host::network::Network as HostNetwork,
    network::Network,
};

mod channel;
mod currency;
mod host;
mod network;

#[derive(Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub(crate) struct Raw {
    host_network: HostNetwork,
    networks: BTreeMap<String, Network>,
    channels: Vec<Channel>,
    #[serde(rename = "definitions")]
    _definitions: Option<Vec<String>>,
}

impl From<Raw> for super::Topology {
    fn from(
        Raw {
            host_network,
            networks,
            channels,
            ..
        }: Raw,
    ) -> Self {
        Self {
            host_network,
            networks,
            channels,
        }
    }
}
