use serde::Deserialize;

use crate::network::Id as Network;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
pub(crate) struct Endpoint {
    network: Network,
    #[serde(rename = "ch")]
    channel_id: super::Id,
}

impl Endpoint {
    #[inline]
    pub const fn network(&self) -> &Network {
        &self.network
    }

    #[inline]
    pub const fn channel_id(&self) -> &super::Id {
        &self.channel_id
    }
}
