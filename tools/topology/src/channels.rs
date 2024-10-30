use std::collections::{BTreeMap, BTreeSet};

use crate::{channel, network};

#[derive(Debug, Clone, PartialEq, Eq)]
#[repr(transparent)]
pub(crate) struct Map<'network, 'channel_id>(
    BTreeMap<&'network network::Id, ConnectedNetworks<'network, 'channel_id>>,
);

impl<'network, 'channel_id> Map<'network, 'channel_id> {
    // TODO convert to the following after upgrade to Rust 1.79+:
    //  ```
    //  const fn new() -> Self { const { Self(BTree::new()) } }
    //  ```
    pub const EMPTY: Self = Self(BTreeMap::new());

    #[inline]
    pub fn get<'self_>(
        &'self_ self,
        network: &network::Id,
    ) -> Option<&'self_ ConnectedNetworks<'network, 'channel_id>> {
        self.0.get(network)
    }
}

impl<'network, 'channel_id> From<MutableMap<'network, 'channel_id>> for Map<'network, 'channel_id> {
    #[inline]
    fn from(MutableMap { map, .. }: MutableMap<'network, 'channel_id>) -> Self {
        map
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MutableMap<'network, 'channel_id> {
    map: Map<'network, 'channel_id>,
    assigned_channel_ids: BTreeMap<&'network network::Id, BTreeSet<&'channel_id channel::Id>>,
}

impl<'network, 'channel_id> MutableMap<'network, 'channel_id> {
    // TODO convert to the following after upgrade to Rust 1.79+:
    //  ```
    //  const fn new() -> Self {
    //      const {
    //          Self {
    //              map: Map::new(),
    //              assigned_channels: BTreeMap::new(),
    //          }
    //      }
    //  }
    //  ```
    pub const EMPTY: Self = Self {
        map: Map::EMPTY,
        assigned_channel_ids: BTreeMap::new(),
    };

    #[inline]
    pub fn get<'self_>(
        &'self_ self,
        network: &network::Id,
    ) -> Option<&'self_ ConnectedNetworks<'network, 'channel_id>> {
        self.map.get(network)
    }

    pub fn insert(
        &mut self,
        a: (&'network network::Id, &'channel_id channel::Id),
        b: (&'network network::Id, &'channel_id channel::Id),
    ) -> Result<(), crate::error::ProcessChannels> {
        let endpoints = [([a.0, b.0], a.1), ([b.0, a.0], b.1)];

        if endpoints.iter().any(|&([source, remote], channel_id)| {
            self.get(source).map_or(false, |connected_to| {
                let Some(assigned_channel_ids) = self.assigned_channel_ids.get(source) else {
                    unreachable!(
                        "Assigned channel IDs set should already exist when \
                            the network exists!"
                    )
                };

                connected_to.get(remote).is_some() || assigned_channel_ids.contains(channel_id)
            })
        }) {
            Err(crate::error::ProcessChannels::DuplicateChannel)
        } else {
            for ([source, remote], channel_id) in endpoints {
                if !self
                    .assigned_channel_ids
                    .entry(source)
                    .or_default()
                    .insert(channel_id)
                {
                    unreachable!("Channel ID shouldn't have already been assigned!");
                }

                if self
                    .map
                    .0
                    .entry(source)
                    .or_insert_with(ConnectedNetworks::new)
                    .0
                    .insert(remote, channel_id)
                    .is_some()
                {
                    unreachable!("Channel endpoint shouldn't have already existed!");
                };
            }

            Ok(())
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[repr(transparent)]
pub(crate) struct ConnectedNetworks<'network, 'channel_id>(
    BTreeMap<&'network network::Id, &'channel_id channel::Id>,
);

impl<'network, 'channel_id> ConnectedNetworks<'network, 'channel_id> {
    const fn new() -> Self {
        Self(BTreeMap::new())
    }

    #[inline]
    pub fn iter(
        &self,
    ) -> impl DoubleEndedIterator<Item = (&&'network network::Id, &&'channel_id channel::Id)> + '_
    {
        self.0.iter()
    }

    pub fn get<'self_>(
        &'self_ self,
        network: &network::Id,
    ) -> Option<&'self_ &'channel_id channel::Id> {
        self.0.get(network)
    }
}
