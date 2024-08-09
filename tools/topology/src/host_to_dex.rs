use std::{
    collections::{BTreeSet, VecDeque},
    mem,
};

use crate::{channel, channels, error, network};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Channel<'channel_id> {
    channel_id: &'channel_id channel::Id,
    counterpart_channel_id: &'channel_id channel::Id,
}

impl<'channel_id> Channel<'channel_id> {
    #[inline]
    pub const fn new(
        channel_id: &'channel_id channel::Id,
        counterpart_channel_id: &'channel_id channel::Id,
    ) -> Self {
        Self {
            channel_id,
            counterpart_channel_id,
        }
    }

    #[inline]
    pub const fn channel_id(&self) -> &channel::Id {
        self.channel_id
    }

    #[inline]
    pub const fn counterpart_channel_id(&self) -> &channel::Id {
        self.counterpart_channel_id
    }
}

pub(crate) fn find_path<'channels_map>(
    channels: &'channels_map channels::Map<'_, '_>,
    host_network: &network::Id,
    dex_network: &network::Id,
) -> Result<Vec<Channel<'channels_map>>, error::CurrencyDefinitions> {
    direct_host_to_dex_path(channels, host_network, dex_network).map_or_else(
        || indirect_host_to_dex_path(channels, host_network, dex_network),
        |channel| Ok(vec![channel]),
    )
}

fn direct_host_to_dex_path<'channels_map>(
    channels: &'channels_map channels::Map<'_, '_>,
    host_network: &network::Id,
    dex_network: &network::Id,
) -> Option<Channel<'channels_map>> {
    channels.get(host_network).and_then(|connected_networks| {
        connected_networks.get(dex_network).map(|&channel_id| {
            let Some(connected_networks) = channels.get(dex_network) else {
                unreachable_counterpart_should_be_filled_in();
            };

            let Some(&counterpart_channel_id) = connected_networks.get(host_network) else {
                unreachable_counterpart_should_be_filled_in();
            };

            Channel::new(channel_id, counterpart_channel_id)
        })
    })
}

struct Path<'network, 'channels_map, 'channels_network, 'channel_id>
where
    'channels_network: 'network,
{
    pub network: &'network network::Id,
    pub connected_networks:
        &'channels_map channels::ConnectedNetworks<'channels_network, 'channel_id>,
    pub walked_channels: Vec<Channel<'channel_id>>,
}

enum TraverseNetworkOutput {
    AlreadyTraversed,
    NewNetwork,
}

fn indirect_host_to_dex_path<'channels_map>(
    channels: &'channels_map channels::Map<'_, '_>,
    host_network: &network::Id,
    dex_network: &network::Id,
) -> Result<Vec<Channel<'channels_map>>, error::CurrencyDefinitions> {
    let mut paths_to_explore = initial_host_to_dex_paths(channels, host_network)?;

    let mut set_traversed = {
        let mut traversed_networks = BTreeSet::from([host_network]);

        move |network| {
            if traversed_networks.insert(network) {
                TraverseNetworkOutput::NewNetwork
            } else {
                TraverseNetworkOutput::AlreadyTraversed
            }
        }
    };

    loop {
        let Some(exploration_path) = paths_to_explore.pop_front() else {
            break Err(error::CurrencyDefinitions::HostNotConnectedToDex);
        };

        if let Some(channels) = explore_path_breadth_first(
            channels,
            dex_network,
            |discovered_path| paths_to_explore.push_back(discovered_path),
            &mut set_traversed,
            exploration_path,
        ) {
            break Ok(channels);
        };
    }
}

fn initial_host_to_dex_paths<'channels_map, 'channels_network, 'channel_id>(
    channels: &'channels_map channels::Map<'channels_network, 'channel_id>,
    host_network: &network::Id,
) -> Result<
    VecDeque<Path<'channels_network, 'channels_map, 'channels_network, 'channel_id>>,
    error::CurrencyDefinitions,
> {
    channels
        .get(host_network)
        .ok_or(error::CurrencyDefinitions::HostNotConnectedToDex)
        .map(|connected_networks| {
            connected_networks
                .iter()
                .map(|(&network, &channel_id)| {
                    let Some(connected_networks) = channels.get(network) else {
                        unreachable_counterpart_should_be_filled_in();
                    };

                    let Some(&counterpart_channel_id) = connected_networks.get(host_network) else {
                        unreachable_counterpart_should_be_filled_in();
                    };

                    Path {
                        network,
                        connected_networks,
                        walked_channels: vec![Channel::new(channel_id, counterpart_channel_id)],
                    }
                })
                .collect()
        })
}

fn explore_path_breadth_first<
    'network,
    'channels_map,
    'channels_network,
    'channel_id,
    DiscoverPath: FnMut(Path<'network, 'channels_map, 'channels_network, 'channel_id>),
    TraverseNetwork: FnMut(&'network network::Id) -> TraverseNetworkOutput,
>(
    channels: &'channels_map channels::Map<'channels_network, 'channel_id>,
    dex_network: &network::Id,
    mut discovered_path: DiscoverPath,
    mut set_traversed: TraverseNetwork,
    mut path: Path<'network, 'channels_map, 'channels_network, 'channel_id>,
) -> Option<Vec<Channel<'channel_id>>>
where
    'channels_network: 'network,
{
    let mut connected_networks = path
        .connected_networks
        .iter()
        .map(|(&connected_network, &chanel_id)| (connected_network, chanel_id))
        .filter(|&(network, _)| {
            matches!(set_traversed(network), TraverseNetworkOutput::NewNetwork)
        });

    let last_endpoint = connected_networks.next_back();

    connected_networks
        .map(|tuple| (false, tuple))
        .chain(last_endpoint.map(|tuple| (true, tuple)))
        .find_map(move |(is_last, (next_network, channel_id))| {
            let Some(connected_networks) = channels.get(next_network) else {
                unreachable_counterpart_should_be_filled_in();
            };

            let Some(&counterpart_channel_id) = connected_networks.get(path.network) else {
                unreachable_counterpart_should_be_filled_in();
            };

            let channel = Channel::new(channel_id, counterpart_channel_id);

            if next_network == dex_network {
                path.walked_channels.push(channel);

                Some(mem::take(&mut path.walked_channels))
            } else {
                let mut walked_channels = if is_last {
                    mem::take(&mut path.walked_channels)
                } else {
                    path.walked_channels.clone()
                };

                walked_channels.push(channel);

                discovered_path(Path {
                    network: next_network,
                    connected_networks,
                    walked_channels,
                });

                None
            }
        })
}

#[cold]
#[inline]
#[track_caller]
fn unreachable_counterpart_should_be_filled_in() -> ! {
    unreachable!(
        "Inverse channel endpoint should be filled in during channels \
            processing!"
    );
}
