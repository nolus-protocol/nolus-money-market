use serde::Deserialize;

use crate::network;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub(crate) struct Endpoint {
    network: network::Id,
    #[serde(rename = "ch")]
    channel_id: super::Id,
}

impl Endpoint {
    #[inline]
    pub const fn network(&self) -> &network::Id {
        &self.network
    }

    #[inline]
    pub const fn channel_id(&self) -> &super::Id {
        &self.channel_id
    }
}
