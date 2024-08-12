use serde::Deserialize;

pub use self::currency_definition::CurrencyDefinition;
use self::{
    channel::Channel, currency::Currency, network::HostNetwork, networks::Networks,
    symbol::Builder as SymbolBuilder,
};

mod channel;
mod channels;
mod currencies;
mod currency;
mod currency_definition;
pub mod error;
mod host_to_dex;
mod network;
mod networks;
mod symbol;

#[derive(Debug, Clone, Deserialize)]
#[serde(from = "self::Raw")]
pub struct Topology {
    host_network: HostNetwork,
    networks: Networks,
    channels: Vec<Channel>,
}

impl Topology {
    pub fn currency_definitions(
        &self,
        dex_network: &str,
    ) -> Result<Vec<CurrencyDefinition>, error::CurrencyDefinitions> {
        let &(dex_network, dex_currencies) = &self
            .networks
            .get_id_and_network(dex_network)
            .map(|(id, network)| (id, network.currencies()))
            .ok_or(error::CurrencyDefinitions::NonExistentDexNetwork)?;

        let channels = self.process_channels()?;

        let mut currencies = vec![];

        currencies.reserve_exact(dex_currencies.len());

        let host_to_dex_path =
            host_to_dex::find_path(&channels, self.host_network.name(), dex_network)?;

        dex_currencies
            .iter()
            .try_for_each(|(ticker, currency)| {
                self.resolve_currency(dex_network, &channels, &host_to_dex_path, ticker, currency)
                    .map(|currency| currencies.push(currency))
            })
            .map(|()| currencies)
            .map_err(Into::into)
    }

    fn process_channels(&self) -> Result<channels::Map<'_, '_>, error::ProcessChannels> {
        let mut channels = channels::MutableMap::EMPTY;

        self.channels
            .iter()
            .try_for_each(|channel| {
                let endpoint_a = channel.a();

                let endpoint_b = channel.b();

                channels.insert(
                    (endpoint_a.network(), endpoint_a.channel_id()),
                    (endpoint_b.network(), endpoint_b.channel_id()),
                )
            })
            .map(|()| channels.into())
    }

    fn resolve_currency(
        &self,
        dex_network: &network::Id,
        channels: &channels::Map<'_, '_>,
        host_to_dex_path: &[host_to_dex::Channel<'_>],
        ticker: &currency::Id,
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

    fn traverse_and_extract_currency_path<'self_, 'network, 'currency>(
        &'self_ self,
        channels: &channels::Map<'_, '_>,
        host_to_dex_path: &[host_to_dex::Channel<'_>],
        dex_symbol: &mut SymbolBuilder,
        traversed_networks: &mut Vec<&'network network::Id>,
        mut currency: &'currency Currency,
    ) -> Result<&'currency currency::Native, error::ResolveCurrency>
    where
        'self_: 'currency,
        'currency: 'network,
    {
        Ok(loop {
            match currency {
                Currency::Native(native) => {
                    break native;
                }
                Currency::Ibc(ibc) if ibc.network() == self.host_network.name() => {
                    if traversed_networks.contains(&ibc.network()) {
                        unreachable!(
                            "Host network should not have been already \
                                traversed!"
                        );
                    }

                    assert_eq!(ibc.currency(), self.host_network.currency().id());

                    traversed_networks.push(self.host_network.name());

                    dex_symbol.add_channel(host_to_dex_path[0].counterpart_channel_id());

                    break self.host_network.currency().native();
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

    fn resolve_non_host_ibc_currency<'self_, 'network, 'currency>(
        &'self_ self,
        channels: &channels::Map<'_, '_>,
        dex_symbol: &mut SymbolBuilder,
        traversed_networks: &mut Vec<&'network network::Id>,
        ibc: &'currency currency::Ibc,
    ) -> Result<&'currency Currency, error::ResolveCurrency>
    where
        'self_: 'currency,
        'currency: 'network,
    {
        if traversed_networks.contains(&ibc.network()) {
            return Err(error::ResolveCurrency::CycleCreated);
        }

        if let Some(&channel_id) = channels
            .get(traversed_networks.last().unwrap_or_else(
                #[inline]
                || unreachable!(),
            ))
            .and_then(|connected_networks| connected_networks.get(ibc.network()))
        {
            dex_symbol.add_channel(channel_id);
        }

        traversed_networks.push(ibc.network());

        self.networks
            .get(ibc.network())
            .ok_or_else(|| error::ResolveCurrency::NoSuchNetwork(ibc.network().as_ref().into()))
            .and_then(|network| {
                network.currencies().get(ibc.currency()).ok_or_else(|| {
                    error::ResolveCurrency::NoSuchCurrency(ibc.currency().as_ref().into())
                })
            })
    }

    fn finalize_currency_resolution(
        &self,
        channels: &channels::Map<'_, '_>,
        host_to_dex_path: &[host_to_dex::Channel<'_>],
        ticker: &currency::Id,
        dex_symbol: SymbolBuilder,
        traversed_networks: &[&network::Id],
        native_currency: &currency::Native,
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
                let &[source_network, remote_network] = networks else {
                    unreachable!("Window slice should be exactly two elements.");
                };

                channels
                    .get(source_network)
                    .and_then(|connected_networks| connected_networks.get(remote_network))
                    .ok_or_else(|| {
                        error::ResolveCurrency::NetworksNotConnected(
                            source_network.as_ref().into(),
                            remote_network.as_ref().into(),
                        )
                    })
                    .map(|&channel_id| bank_symbol.add_channel(channel_id))
            })
            .map(|()| {
                CurrencyDefinition::new(
                    ticker.as_ref().into(),
                    bank_symbol.add_symbol(native_currency.symbol()),
                    dex_symbol.add_symbol(native_currency.symbol()),
                    native_currency.decimal_digits(),
                )
            })
    }

    fn get_bank_symbol_traversal_start(
        &self,
        host_to_dex_path: &[host_to_dex::Channel<'_>],
        traversed_networks: &[&network::Id],
        bank_symbol: &mut SymbolBuilder,
    ) -> usize {
        traversed_networks
            .iter()
            .enumerate()
            .find_map(|(index, &network)| (*network == *self.host_network.name()).then_some(index))
            .unwrap_or_else(|| {
                host_to_dex_path
                    .iter()
                    .for_each(|&channel| bank_symbol.add_channel(channel.channel_id()));

                0
            })
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
struct Raw {
    host_network: HostNetwork,
    networks: Networks,
    channels: Vec<Channel>,
    #[serde(rename = "definitions")]
    _definitions: Option<Vec<String>>,
}

impl From<Raw> for Topology {
    fn from(
        Raw {
            host_network,
            networks,
            channels,
            ..
        }: Raw,
    ) -> Self {
        Self {
            host_network,
            networks,
            channels,
        }
    }
}
