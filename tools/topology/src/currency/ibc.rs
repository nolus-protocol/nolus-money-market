use serde::Deserialize;

use crate::{network, skippable::Skippable};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub(crate) struct Ibc {
    network: network::Id,
    currency: super::Id,
    #[serde(default)]
    override_symbol: Skippable<super::Id>,
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

    #[inline]
    pub const fn overriden_symbol(&self) -> Option<&super::Id> {
        match self.override_symbol {
            Skippable::Skipped => None,
            Skippable::Some(ref symbol) => Some(symbol),
        }
    }
}
