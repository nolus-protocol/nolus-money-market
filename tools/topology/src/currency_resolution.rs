use crate::{
    channels,
    currency::{self, Currency},
    currency_definition::CurrencyDefinition,
    error, host_to_dex, network,
    networks::Networks,
    symbol, Topology,
};

impl Topology {
    pub(super) fn resolve_currency(
        &self,
        dex_network: &network::Id,
        channels: &channels::Map<'_, '_>,
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

    fn resolve_dex_side<'self_, 'network, 'currency>(
        &'self_ self,
        dex_network: &'network network::Id,
        channels: &channels::Map<'network, '_>,
        host_to_dex_path: &[host_to_dex::Channel<'_>],
        currency: &'currency Currency,
    ) -> Result<DexResolvedCurrency<'network, 'currency>, error::ResolveCurrency>
    where
        'self_: 'currency,
        'currency: 'network,
    {
        // Shadow to shrink reference's lifetime.
        let mut currency = currency;

        let mut traversed_networks = vec![dex_network];

        let mut dex_symbol = symbol::Builder::NEW;

        Ok(loop {
            match currency {
                Currency::Native(native) => {
                    break DexResolvedCurrency {
                        traversed_networks,
                        dex_symbol,
                        currency: native,
                        host_local: false,
                    };
                }
                Currency::Ibc(ibc) if ibc.network() == self.host_network.name() => {
                    break Self::resolve_host_currency(
                        &self.host_network,
                        host_to_dex_path,
                        traversed_networks,
                        dex_symbol,
                        ibc,
                    );
                }
                Currency::Ibc(ibc) => {
                    ResolvedNonHostIbcCurrency {
                        traversed_networks,
                        dex_symbol,
                        currency,
                    } = Self::resolve_non_host_ibc_currency(
                        &self.networks,
                        channels,
                        traversed_networks,
                        dex_symbol,
                        ibc,
                    )?;
                }
            }
        })
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

    fn resolve_non_host_ibc_currency<'networks, 'network, 'currency, 'channel_id>(
        networks: &'networks Networks,
        channels: &channels::Map<'network, 'channel_id>,
        mut traversed_networks: Vec<&'network network::Id>,
        mut dex_symbol: symbol::Builder,
        ibc: &'currency currency::Ibc,
    ) -> Result<ResolvedNonHostIbcCurrency<'network, 'currency>, error::ResolveCurrency>
    where
        'networks: 'currency,
        'currency: 'network,
    {
        if traversed_networks.contains(&ibc.network()) {
            Err(error::ResolveCurrency::CycleCreated)
        } else {
            if let Some(&channel_id) = channels
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
        channels: &channels::Map<'_, '_>,
        host_to_dex_path: &[host_to_dex::Channel<'_>],
        ticker: &currency::Id,
    ) -> Result<HostLocality, error::ResolveCurrency> {
        let mut bank_symbol = symbol::Builder::NEW;

        let bank_symbol_traversal_start = self
            .traversed_networks
            .iter()
            .enumerate()
            .find_map(|(index, &network)| (*network == *host_network.name()).then_some(index))
            .unwrap_or_else(|| {
                host_to_dex_path
                    .iter()
                    .for_each(|&channel| bank_symbol.add_channel(channel.channel_id()));

                0
            });

        self.traversed_networks[bank_symbol_traversal_start..]
            .windows(2)
            .try_for_each(|networks| {
                let &[source_network, remote_network] = networks else {
                    unreachable!("Window slice should be exactly two elements.");
                };

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
                    .map(|&channel_id| bank_symbol.add_channel(channel_id))
            })
            .map(|()| {
                let currency_definition = CurrencyDefinition::new(
                    ticker.as_ref().into(),
                    bank_symbol.add_symbol(self.currency.symbol()),
                    self.dex_symbol.add_symbol(self.currency.symbol()),
                    self.currency.decimal_digits(),
                );

                if self.host_local {
                    HostLocality::Local(currency_definition)
                } else {
                    HostLocality::Remote(currency_definition)
                }
            })
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
