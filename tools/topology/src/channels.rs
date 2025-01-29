use std::collections::BTreeMap;

use serde::Deserialize;

use crate::{channel, network};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[repr(transparent)]
#[serde(transparent)]
pub(crate) struct Channels(BTreeMap<network::Id, ConnectedNetworks>);

impl Channels {
    #[inline]
    pub fn get<'self_>(&'self_ self, network: &network::Id) -> Option<&'self_ ConnectedNetworks> {
        self.0.get(network)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[repr(transparent)]
#[serde(transparent)]
pub(crate) struct ConnectedNetworks(BTreeMap<network::Id, channel::Id>);

impl ConnectedNetworks {
    #[inline]
    pub fn iter(
        &self,
    ) -> impl DoubleEndedIterator<Item = (&network::Id, &channel::Id)> + '_ + use<'_> {
        self.0.iter()
    }

    pub fn get<'self_>(&'self_ self, network: &network::Id) -> Option<&'self_ channel::Id> {
        self.0.get(network)
    }
}
