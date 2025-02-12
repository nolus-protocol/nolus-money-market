use std::collections::BTreeMap;

use serde::Deserialize;

use crate::network::{self, Network};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub(crate) struct Networks(BTreeMap<network::Id, Network>);

impl Networks {
    #[inline]
    pub fn get<'self_>(&'self_ self, network: &network::Id) -> Option<&'self_ Network> {
        self.0.get(network)
    }

    #[inline]
    pub fn get_id_and_network<'self_>(
        &'self_ self,
        network: &str,
    ) -> Option<(&'self_ network::Id, &'self_ Network)> {
        self.0.get_key_value(network)
    }
}
