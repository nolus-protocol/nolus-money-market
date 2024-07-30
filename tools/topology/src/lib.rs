use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::Deserialize;

pub use self::currency_definition::CurrencyDefinition;
use self::{
    inner_structure::{Channel, Currency, HostNetwork, IbcCurrency, NativeCurrency, Network},
    symbol::Builder,
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
        channels: &BTreeMap<&str, BTreeMap<&str, &'r str>>,
        host_network: &str,
        dex_network: &str,
    ) -> Result<Box<[HostToDexPathChannel<'r>]>, error::CurrencyDefinitions> {
        let mut endpoints_deque: VecDeque<_> = channels
            .get(host_network)
            .ok_or(error::CurrencyDefinitions::HostNotConnectedToDex)?
            .iter()
            .map(|(&network, &endpoint)| {
                let Some((endpoints, inverse_endpoint)) =
                    channels.get(&network).and_then(|endpoints| {
                        endpoints
                            .get(host_network)
                            .map(|&inverse_endpoint| (endpoints, inverse_endpoint))
                    })
                else {
                    unreachable!("Inverse channel endpoint has to be defined!")
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
            .collect();

        let mut traversed_networks = BTreeSet::from([host_network]);

        Ok('find_connection: loop {
            let Some((network, endpoints, mut walked_endpoints)) = endpoints_deque.pop_front()
            else {
                return Err(error::CurrencyDefinitions::HostNotConnectedToDex);
            };

            if network == dex_network {
                break 'find_connection walked_endpoints;
            }

            let mut endpoints = endpoints
                .iter()
                .filter(|&(network, _)| traversed_networks.insert(network));

            let Some((&next_network, &endpoint)) = endpoints.next() else {
                continue;
            };

            if next_network != dex_network {
                for (&next_network, &endpoint) in endpoints {
                    let Some(endpoints) = channels.get(next_network) else {
                        unreachable!()
                    };

                    let Some(&inverse_endpoint) = endpoints.get(network) else {
                        unreachable!()
                    };

                    let host_dex_path_channel = HostToDexPathChannel {
                        endpoint,
                        inverse_endpoint,
                    };

                    if next_network == dex_network {
                        walked_endpoints.push(host_dex_path_channel);

                        break 'find_connection walked_endpoints;
                    }

                    let mut walked_endpoints = walked_endpoints.clone();

                    walked_endpoints.push(host_dex_path_channel);

                    endpoints_deque.push_back((next_network, endpoints, walked_endpoints));
                }
            }

            let Some(endpoints) = channels.get(next_network) else {
                unreachable!()
            };

            let Some(&inverse_endpoint) = endpoints.get(network) else {
                unreachable!()
            };

            walked_endpoints.push(HostToDexPathChannel {
                endpoint,
                inverse_endpoint,
            });

            if next_network == dex_network {
                break 'find_connection walked_endpoints;
            }

            endpoints_deque.push_back((next_network, endpoints, walked_endpoints));
        }
        .into_boxed_slice())
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
        let mut dex_symbol = Builder::NEW;

        let mut traversed_networks = vec![dex_network];

        let mut currency = currency;

        let native_currency = 'resolve_currency: loop {
            match currency {
                Currency::Native(native) => {
                    break 'resolve_currency native;
                }
                Currency::Ibc(ibc) if ibc.network == self.host_network.name => {
                    if traversed_networks.contains(&&*ibc.network) {
                        unreachable!("Host network should not have been already traversed!");
                    }

                    assert_eq!(ibc.currency, self.host_network.currency.id);

                    traversed_networks.push(&self.host_network.name);

                    dex_symbol.add_channel(host_to_dex_path[0].inverse_endpoint);

                    break 'resolve_currency &self.host_network.currency.native;
                }
                Currency::Ibc(ibc) => {
                    currency = self.resolve_non_host_ibc_currency(
                        channels,
                        &mut dex_symbol,
                        &mut traversed_networks,
                        ibc,
                    )?;
                }
            }
        };

        self.finalize_currency_resolution(
            channels,
            host_to_dex_path,
            ticker,
            dex_symbol,
            &traversed_networks,
            native_currency,
        )
    }

    fn resolve_non_host_ibc_currency<'r, 't>(
        &'r self,
        channels: &BTreeMap<&str, BTreeMap<&str, &str>>,
        dex_symbol: &mut Builder,
        traversed_networks: &mut Vec<&'t str>,
        ibc: &'t IbcCurrency,
    ) -> Result<&'r Currency, error::ResolveCurrency> {
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
        dex_symbol: Builder,
        traversed_networks: &[&str],
        native_currency: &NativeCurrency,
    ) -> Result<CurrencyDefinition, error::ResolveCurrency> {
        let mut bank_symbol = Builder::NEW;

        let bank_symbol_traversal_start = traversed_networks
            .iter()
            .enumerate()
            .find_map(|(index, &network)| (*network == *self.host_network.name).then_some(index))
            .unwrap_or_else(|| {
                host_to_dex_path
                    .iter()
                    .for_each(|&channel| bank_symbol.add_channel(channel.endpoint));

                0
            });

        traversed_networks[bank_symbol_traversal_start..]
            .windows(2)
            .try_for_each(|networks_window| {
                let [from, to] = networks_window else {
                    unreachable!()
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
}

#[derive(Clone, Copy)]
struct HostToDexPathChannel<'r> {
    endpoint: &'r str,
    inverse_endpoint: &'r str,
}
