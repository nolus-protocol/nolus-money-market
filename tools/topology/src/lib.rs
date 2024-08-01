use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    mem,
};

use serde::Deserialize;

pub use self::currency_definition::CurrencyDefinition;
use self::{
    inner_structure::{Channel, Currency, HostNetwork, IbcCurrency, NativeCurrency, Network},
    symbol::Builder as SymbolBuilder,
};

mod currency_definition;
pub mod error;
mod inner_structure;
mod symbol;

#[derive(Debug, Clone, Deserialize)]
#[serde(from = "self::inner_structure::Raw")]
pub struct Topology {
    host_network: HostNetwork,
    networks: BTreeMap<Box<str>, Network>,
    channels: Box<[Channel]>,
}

impl Topology {
    pub fn currency_definitions(
        &self,
        dex_network: &str,
    ) -> Result<Box<[CurrencyDefinition]>, error::CurrencyDefinitions> {
        let dex_currencies = &self
            .networks
            .get(dex_network)
            .ok_or(error::CurrencyDefinitions::NonExistentDexNetwork)?
            .currencies;

        let channels = self.process_channels()?;

        let host_to_dex_path =
            Self::host_to_dex_path(&channels, &self.host_network.name, dex_network)?;

        let mut currencies = vec![];

        currencies.reserve_exact(dex_currencies.len());

        dex_currencies
            .iter()
            .try_for_each(|(ticker, currency)| {
                self.resolve_currency(dex_network, &channels, &host_to_dex_path, ticker, currency)
                    .map(|currency| currencies.push(currency))
            })
            .map(|()| currencies.into_boxed_slice())
            .map_err(Into::into)
    }

    fn host_to_dex_path<'r>(
        channels: &'r BTreeMap<&str, BTreeMap<&str, &str>>,
        host_network: &str,
        dex_network: &str,
    ) -> Result<Box<[HostToDexPathChannel<'r>]>, error::CurrencyDefinitions> {
        Self::direct_host_to_dex_path(channels, host_network, dex_network).map_or_else(
            || Self::indirect_host_to_dex_path(channels, host_network, dex_network),
            |channel| Ok(Box::new([channel]) as Box<[_]>),
        )
    }

    fn direct_host_to_dex_path<'r>(
        channels: &'r BTreeMap<&str, BTreeMap<&str, &str>>,
        host_network: &str,
        dex_network: &str,
    ) -> Option<HostToDexPathChannel<'r>> {
        channels.get(host_network).and_then(|connected_networks| {
            connected_networks.get(dex_network).map(|&endpoint| {
                let Some(connected_networks) = channels.get(dex_network) else {
                    Self::unreachable_inverse_should_be_filled_in();
                };

                let Some(&inverse_endpoint) = connected_networks.get(host_network) else {
                    Self::unreachable_inverse_should_be_filled_in();
                };

                HostToDexPathChannel {
                    endpoint,
                    inverse_endpoint,
                }
            })
        })
    }

    fn indirect_host_to_dex_path<'r>(
        channels: &'r BTreeMap<&str, BTreeMap<&str, &str>>,
        host_network: &str,
        dex_network: &str,
    ) -> Result<Box<[HostToDexPathChannel<'r>]>, error::CurrencyDefinitions> {
        let mut endpoints_deque = Self::initial_host_to_dex_paths(channels, host_network)?;

        let mut traversed_networks = BTreeSet::from([host_network]);

        loop {
            let Some((network, endpoints, mut walked_channels)) = endpoints_deque.pop_front()
            else {
                break Err(error::CurrencyDefinitions::HostNotConnectedToDex);
            };

            if let Some(path) = Self::explore_path_breath_first(
                channels,
                dex_network,
                &mut endpoints_deque,
                &mut traversed_networks,
                network,
                endpoints,
                &mut walked_channels,
            ) {
                break Ok(path);
            }
        }
    }

    fn explore_path_breath_first<
        'channels_map,
        'source_network,
        'connected_network,
        'endpoint,
        'network,
    >(
        channels: &'channels_map BTreeMap<
            &'source_network str,
            BTreeMap<&'connected_network str, &'endpoint str>,
        >,
        dex_network: &str,
        endpoints_deque: &mut VecDeque<(
            &'network str,
            &'channels_map BTreeMap<&'connected_network str, &'endpoint str>,
            Vec<HostToDexPathChannel<'channels_map>>,
        )>,
        traversed_networks: &mut BTreeSet<&'network str>,
        network: &'network str,
        endpoints: &BTreeMap<&'connected_network str, &'endpoint str>,
        walked_channels: &mut Vec<HostToDexPathChannel<'endpoint>>,
    ) -> Option<Box<[HostToDexPathChannel<'channels_map>]>>
    where
        'endpoint: 'channels_map,
        'connected_network: 'network,
    {
        let mut endpoints = endpoints
            .iter()
            .map(|(&connected_network, &endpoint)| (connected_network, endpoint))
            .filter(|&(network, _)| traversed_networks.insert(network));

        let last = endpoints.next_back();

        endpoints
            .map(|tuple| (false, tuple))
            .chain(last.map(|tuple| (true, tuple)))
            .find_map(move |(is_last, (next_network, endpoint))| {
                let Some(endpoints) = channels.get(next_network) else {
                    Self::unreachable_inverse_should_be_filled_in();
                };

                let Some(&inverse_endpoint) = endpoints.get(network) else {
                    Self::unreachable_inverse_should_be_filled_in();
                };

                let channel = HostToDexPathChannel {
                    endpoint,
                    inverse_endpoint,
                };

                if next_network == dex_network {
                    walked_channels.push(channel);

                    Some(mem::replace(walked_channels, vec![]).into_boxed_slice())
                } else {
                    let mut walked_channels = if is_last {
                        mem::replace(walked_channels, vec![])
                    } else {
                        walked_channels.clone()
                    };

                    walked_channels.push(channel);

                    endpoints_deque.push_back((next_network, endpoints, walked_channels));

                    None
                }
            })
    }

    fn initial_host_to_dex_paths<'r, 't, 'u>(
        channels: &'r BTreeMap<&str, BTreeMap<&'t str, &'u str>>,
        host_network: &str,
    ) -> Result<
        VecDeque<(
            &'r str,
            &'r BTreeMap<&'t str, &'u str>,
            Vec<HostToDexPathChannel<'r>>,
        )>,
        error::CurrencyDefinitions,
    > {
        channels
            .get(host_network)
            .ok_or(error::CurrencyDefinitions::HostNotConnectedToDex)
            .map(|connected_networks| {
                connected_networks
                    .iter()
                    .map(|(&network, &endpoint)| {
                        let Some(endpoints) = channels.get(&network) else {
                            Self::unreachable_inverse_should_be_filled_in();
                        };

                        let Some(&inverse_endpoint) = endpoints.get(host_network) else {
                            Self::unreachable_inverse_should_be_filled_in();
                        };

                        (
                            network,
                            endpoints,
                            vec![HostToDexPathChannel {
                                endpoint,
                                inverse_endpoint,
                            }],
                        )
                    })
                    .collect()
            })
    }

    #[cold]
    #[inline]
    #[track_caller]
    fn unreachable_inverse_should_be_filled_in() -> ! {
        unreachable!(
            "Inverse channel endpoint should be filled in during channels \
                processing!"
        );
    }

    fn process_channels(
        &self,
    ) -> Result<BTreeMap<&str, BTreeMap<&str, &str>>, error::ProcessChannels> {
        let mut channels: BTreeMap<_, BTreeMap<_, _>> = BTreeMap::new();

        let mut assigned_channels_set: BTreeMap<_, BTreeSet<_>> = BTreeMap::new();

        self.channels
            .iter()
            .try_for_each(|channel| {
                [
                    (&channel.a, &*channel.b.network),
                    (&channel.b, &*channel.a.network),
                ]
                .into_iter()
                .try_for_each(|(endpoint, remote_network)| {
                    if assigned_channels_set
                        .entry(&*endpoint.network)
                        .or_default()
                        .insert(&*endpoint.ch)
                        && channels
                            .entry(&*endpoint.network)
                            .or_default()
                            .insert(remote_network, &*endpoint.ch)
                            .is_none()
                    {
                        Ok(())
                    } else {
                        Err(error::ProcessChannels::DuplicateChannel)
                    }
                })
            })
            .map(|()| channels)
    }

    fn resolve_currency(
        &self,
        dex_network: &str,
        channels: &BTreeMap<&str, BTreeMap<&str, &str>>,
        host_to_dex_path: &[HostToDexPathChannel<'_>],
        ticker: &str,
        currency: &Currency,
    ) -> Result<CurrencyDefinition, error::ResolveCurrency> {
        let mut dex_symbol = SymbolBuilder::NEW;

        let mut traversed_networks = vec![dex_network];

        let native_currency = self.traverse_and_extract_currency_path(
            channels,
            host_to_dex_path,
            &mut dex_symbol,
            &mut traversed_networks,
            currency,
        )?;

        self.finalize_currency_resolution(
            channels,
            host_to_dex_path,
            ticker,
            dex_symbol,
            &traversed_networks,
            native_currency,
        )
    }

    fn traverse_and_extract_currency_path<'self_, 'currency>(
        &'self_ self,
        channels: &BTreeMap<&str, BTreeMap<&str, &str>>,
        host_to_dex_path: &[HostToDexPathChannel],
        dex_symbol: &mut SymbolBuilder,
        traversed_networks: &mut Vec<&'currency str>,
        mut currency: &'currency Currency,
    ) -> Result<&'currency NativeCurrency, error::ResolveCurrency>
    where
        'self_: 'currency,
    {
        Ok(loop {
            match currency {
                Currency::Native(native) => {
                    break native;
                }
                Currency::Ibc(ibc) if ibc.network == self.host_network.name => {
                    if traversed_networks.contains(&&*ibc.network) {
                        unreachable!(
                            "Host network should not have been already \
                                traversed!"
                        );
                    }

                    assert_eq!(ibc.currency, self.host_network.currency.id);

                    traversed_networks.push(&self.host_network.name);

                    dex_symbol.add_channel(host_to_dex_path[0].inverse_endpoint);

                    break &self.host_network.currency.native;
                }
                Currency::Ibc(ibc) => {
                    currency = self.resolve_non_host_ibc_currency(
                        channels,
                        dex_symbol,
                        traversed_networks,
                        ibc,
                    )?;
                }
            }
        })
    }

    fn resolve_non_host_ibc_currency<'self_, 'currency>(
        &'self_ self,
        channels: &BTreeMap<&str, BTreeMap<&str, &str>>,
        dex_symbol: &mut SymbolBuilder,
        traversed_networks: &mut Vec<&'currency str>,
        ibc: &'currency IbcCurrency,
    ) -> Result<&'self_ Currency, error::ResolveCurrency> {
        if traversed_networks.contains(&&*ibc.network) {
            return Err(error::ResolveCurrency::CycleCreated);
        }

        if let Some(&endpoint) = channels
            .get(traversed_networks.last().unwrap_or_else(
                #[inline]
                || unreachable!(),
            ))
            .and_then(|endpoints| endpoints.get(&*ibc.network))
        {
            dex_symbol.add_channel(endpoint);
        }

        traversed_networks.push(&ibc.network);

        self.networks
            .get(&*ibc.network)
            .ok_or_else(|| error::ResolveCurrency::NoSuchNetwork((&*ibc.network).into()))
            .and_then(|network| {
                network
                    .currencies
                    .get(&*ibc.currency)
                    .ok_or_else(|| error::ResolveCurrency::NoSuchCurrency((&*ibc.currency).into()))
            })
    }

    fn finalize_currency_resolution(
        &self,
        channels: &BTreeMap<&str, BTreeMap<&str, &str>>,
        host_to_dex_path: &[HostToDexPathChannel<'_>],
        ticker: &str,
        dex_symbol: SymbolBuilder,
        traversed_networks: &[&str],
        native_currency: &NativeCurrency,
    ) -> Result<CurrencyDefinition, error::ResolveCurrency> {
        let mut bank_symbol = SymbolBuilder::NEW;

        let bank_symbol_traversal_start = self.get_bank_symbol_traversal_start(
            host_to_dex_path,
            traversed_networks,
            &mut bank_symbol,
        );

        traversed_networks[bank_symbol_traversal_start..]
            .windows(2)
            .try_for_each(|networks| {
                let [from, to] = networks else {
                    unreachable!("Window slice should be exactly two elements.");
                };

                channels
                    .get(from)
                    .and_then(|endpoints| endpoints.get(to))
                    .ok_or_else(|| {
                        error::ResolveCurrency::NetworksNotConnected((*from).into(), (*to).into())
                    })
                    .map(|&endpoint| bank_symbol.add_channel(endpoint))
            })
            .map(|()| {
                CurrencyDefinition::new(
                    ticker.into(),
                    bank_symbol.add_symbol(&native_currency.symbol),
                    dex_symbol.add_symbol(&native_currency.symbol),
                    native_currency.decimal_digits,
                )
            })
    }

    fn get_bank_symbol_traversal_start(
        &self,
        host_to_dex_path: &[HostToDexPathChannel],
        traversed_networks: &[&str],
        bank_symbol: &mut SymbolBuilder,
    ) -> usize {
        traversed_networks
            .iter()
            .enumerate()
            .find_map(|(index, &network)| (*network == *self.host_network.name).then_some(index))
            .unwrap_or_else(|| {
                host_to_dex_path
                    .iter()
                    .for_each(|&channel| bank_symbol.add_channel(channel.endpoint));

                0
            })
    }
}

#[derive(Clone, Copy)]
struct HostToDexPathChannel<'r> {
    endpoint: &'r str,
    inverse_endpoint: &'r str,
}
