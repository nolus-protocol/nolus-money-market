use std::ops::ControlFlow;

use crate::{
    Topology,
    channels::Channels,
    currency::{self, Currency},
    currency_definition::CurrencyDefinition,
    error, host_to_dex, network,
    networks::Networks,
    symbol,
};

impl Topology {
    pub(super) fn resolve_currency(
        &self,
        dex_network: &network::Id,
        channels: &Channels,
        host_to_dex_path: &[host_to_dex::Channel<'_>],
        ticker: &currency::Id,
        currency: &Currency,
    ) -> Result<HostLocality, error::ResolveCurrency> {
        self.resolve_dex_side(dex_network, channels, host_to_dex_path, currency)
            .and_then(|dex_resolved| {
                dex_resolved.resolve_bank_side(
                    &self.host_network,
                    channels,
                    host_to_dex_path,
                    ticker,
                )
            })
    }

    fn resolve_dex_side<'self_, 'network, 'channels, 'currency>(
        &'self_ self,
        dex_network: &'network network::Id,
        channels: &'channels Channels,
        host_to_dex_path: &[host_to_dex::Channel<'_>],
        currency: &'currency Currency,
    ) -> Result<DexResolvedCurrency<'network, 'currency>, error::ResolveCurrency>
    where
        'self_: 'currency,
        'channels: 'network,
        'currency: 'network,
    {
        try_trampoline(
            ResolvedNonHostIbcCurrency {
                traversed_networks: vec![dex_network],
                dex_symbol: symbol::Builder::NEW,
                currency,
            },
            |ResolvedNonHostIbcCurrency {
                 traversed_networks,
                 dex_symbol,
                 currency,
             }| {
                match currency {
                    Currency::Native(native) => Ok(ControlFlow::Break(DexResolvedCurrency {
                        traversed_networks,
                        dex_symbol,
                        currency: native,
                        host_local: false,
                    })),
                    Currency::Ibc(ibc) if ibc.network() == self.host_network.name() => {
                        Ok(ControlFlow::Break(Self::resolve_host_currency(
                            &self.host_network,
                            host_to_dex_path,
                            traversed_networks,
                            dex_symbol,
                            ibc,
                        )))
                    }
                    Currency::Ibc(ibc) => Self::resolve_non_host_ibc_currency(
                        &self.networks,
                        channels,
                        traversed_networks,
                        dex_symbol,
                        ibc,
                    )
                    .map(ControlFlow::Continue),
                }
            },
        )
    }

    fn resolve_host_currency<'host_network, 'network, 'currency>(
        host_network: &'host_network network::Host,
        host_to_dex_path: &[host_to_dex::Channel<'_>],
        mut traversed_networks: Vec<&'network network::Id>,
        mut dex_symbol: symbol::Builder,
        ibc: &'currency currency::Ibc,
    ) -> DexResolvedCurrency<'network, 'currency>
    where
        'host_network: 'currency,
        'currency: 'network,
    {
        if traversed_networks.contains(&ibc.network()) {
            unreachable!("Host network should not have been already traversed!",);
        }

        assert_eq!(ibc.currency(), host_network.currency().id());

        traversed_networks.push(host_network.name());

        dex_symbol.add_channel(host_to_dex_path[0].counterpart_channel_id());

        DexResolvedCurrency {
            traversed_networks,
            dex_symbol,
            currency: host_network.currency().native(),
            host_local: true,
        }
    }

    fn resolve_non_host_ibc_currency<'networks, 'network, 'channels, 'currency>(
        networks: &'networks Networks,
        channels: &'channels Channels,
        mut traversed_networks: Vec<&'network network::Id>,
        mut dex_symbol: symbol::Builder,
        ibc: &'currency currency::Ibc,
    ) -> Result<ResolvedNonHostIbcCurrency<'network, 'currency>, error::ResolveCurrency>
    where
        'networks: 'network + 'currency,
        'channels: 'network,
    {
        if traversed_networks.contains(&ibc.network()) {
            Err(error::ResolveCurrency::CycleCreated)
        } else {
            if let Some(channel_id) = channels
                .get(traversed_networks.last().unwrap_or_else(
                    #[inline]
                    || {
                        unreachable!(
                            "Traversed networks list has to contain at least \
                                the starting network!",
                        )
                    },
                ))
                .and_then(|connected_networks| connected_networks.get(ibc.network()))
            {
                dex_symbol.add_channel(channel_id);
            }

            traversed_networks.push(ibc.network());

            networks
                .get(ibc.network())
                .ok_or_else(
                    #[cold]
                    || error::ResolveCurrency::NoSuchNetwork(ibc.network().as_ref().into()),
                )
                .and_then(|network| {
                    network
                        .currencies()
                        .get(ibc.currency())
                        .map(|currency| ResolvedNonHostIbcCurrency {
                            traversed_networks,
                            dex_symbol,
                            currency,
                        })
                        .ok_or_else(
                            #[cold]
                            || {
                                error::ResolveCurrency::NoSuchCurrency(
                                    ibc.currency().as_ref().into(),
                                )
                            },
                        )
                })
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum HostLocality {
    Local(CurrencyDefinition),
    Remote(CurrencyDefinition),
}

struct DexResolvedCurrency<'network, 'currency>
where
    'currency: 'network,
{
    traversed_networks: Vec<&'network network::Id>,
    dex_symbol: symbol::Builder,
    currency: &'currency currency::Native,
    host_local: bool,
}

impl<'network, 'currency> DexResolvedCurrency<'network, 'currency>
where
    'currency: 'network,
{
    fn resolve_bank_side(
        self,
        host_network: &network::Host,
        channels: &Channels,
        host_to_dex_path: &[host_to_dex::Channel<'_>],
        ticker: &currency::Id,
    ) -> Result<HostLocality, error::ResolveCurrency> {
        let (bank_symbol, mut network_windows) = Self::initial_bank_symbol_and_path(
            &self.traversed_networks,
            host_network,
            host_to_dex_path,
        );

        network_windows
            .try_fold(bank_symbol, Self::fold_bank_side_in_windows(channels))
            .map(|bank_symbol| {
                CurrencyDefinition::new(
                    ticker.as_ref().into(),
                    bank_symbol.add_symbol(self.currency.symbol()),
                    self.dex_symbol.add_symbol(self.currency.symbol()),
                    self.currency.decimal_digits(),
                )
            })
            .map(|currency_definition| {
                if self.host_local {
                    HostLocality::Local(currency_definition)
                } else {
                    HostLocality::Remote(currency_definition)
                }
            })
    }

    fn initial_bank_symbol_and_path<'traversed_networks, 'traversed_network_id>(
        traversed_networks: &'traversed_networks [&'traversed_network_id network::Id],
        host_network: &network::Host,
        host_to_dex_path: &[host_to_dex::Channel<'_>],
    ) -> (
        symbol::Builder,
        impl Iterator<Item = [&'traversed_network_id network::Id; 2]>
        + use<'traversed_networks, 'traversed_network_id>,
    ) {
        let mut bank_symbol = symbol::Builder::NEW;

        let traversal_start = traversed_networks
            .iter()
            .enumerate()
            .find_map(|(index, &network)| (*network == *host_network.name()).then_some(index))
            .unwrap_or_else(|| {
                host_to_dex_path
                    .iter()
                    .for_each(|&channel| bank_symbol.add_channel(channel.channel_id()));

                0
            });

        (
            bank_symbol,
            // TODO [feature=array_windows]
            //  PR: https://github.com/rust-lang/rust/issues/75027
            //  Change `windows` to `array_windows` and remove `unreachable!`
            //  use.
            traversed_networks[traversal_start..]
                .windows(2)
                .map(|networks| {
                    networks.try_into().unwrap_or_else(
                        #[inline]
                        |error| {
                            unreachable!(
                                "Window slice should be exactly two elements! \
                                Error: {error:?}"
                            )
                        },
                    )
                }),
        )
    }

    fn fold_bank_side_in_windows<'channels, 'network_id>(
        channels: &'channels Channels,
    ) -> impl FnMut(
        symbol::Builder,
        [&'network_id network::Id; 2],
    ) -> Result<symbol::Builder, error::ResolveCurrency>
    + use<'channels, 'network_id> {
        |mut bank_symbol, [source_network, remote_network]| {
            channels
                .get(source_network)
                .and_then(|connected_networks| connected_networks.get(remote_network))
                .ok_or_else(
                    #[cold]
                    || {
                        error::ResolveCurrency::NetworksNotConnected(
                            source_network.as_ref().into(),
                            remote_network.as_ref().into(),
                        )
                    },
                )
                .map(|channel_id| {
                    bank_symbol.add_channel(channel_id);
                })
                .map(|()| bank_symbol)
        }
    }
}

struct ResolvedNonHostIbcCurrency<'network, 'currency>
where
    'currency: 'network,
{
    traversed_networks: Vec<&'network network::Id>,
    dex_symbol: symbol::Builder,
    currency: &'currency Currency,
}

fn trampoline<Continue, Break, F>(mut value: Continue, mut f: F) -> Break
where
    F: FnMut(Continue) -> ControlFlow<Break, Continue>,
{
    loop {
        match f(value) {
            ControlFlow::Continue(continue_with) => value = continue_with,
            ControlFlow::Break(break_with) => break break_with,
        }
    }
}

fn try_trampoline<F, Continue, Break, Error>(value: Continue, mut f: F) -> Result<Break, Error>
where
    F: FnMut(Continue) -> Result<ControlFlow<Break, Continue>, Error>,
{
    trampoline(value, move |value| match f(value) {
        Ok(ControlFlow::Continue(continue_with)) => ControlFlow::Continue(continue_with),
        Ok(ControlFlow::Break(break_with)) => ControlFlow::Break(Ok(break_with)),
        Err(error) => ControlFlow::Break(Err(error)),
    })
}
