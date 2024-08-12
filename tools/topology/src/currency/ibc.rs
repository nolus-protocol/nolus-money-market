use serde::Deserialize;

use crate::network;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub(crate) struct Ibc {
    network: network::Id,
    currency: super::Id,
}

impl Ibc {
    #[inline]
    pub const fn network(&self) -> &network::Id {
        &self.network
    }

    #[inline]
    pub const fn currency(&self) -> &super::Id {
        &self.currency
    }
}
