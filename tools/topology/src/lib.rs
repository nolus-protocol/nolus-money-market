use serde::Deserialize;

use self::{channel::Channel, currency_resolution::HostLocality, networks::Networks};
pub use self::{currency_definition::CurrencyDefinition, symbol::Symbol};

mod channel;
mod channels;
mod currencies;
pub mod currency;
mod currency_definition;
mod currency_resolution;
pub mod dex;
pub mod error;
mod host_to_dex;
mod network;
mod networks;
pub mod swap_pairs;
mod symbol;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct Topology {
    host_network: network::Host,
    networks: Networks,
    channels: Vec<Channel>,
}

impl Topology {
    pub fn currency_definitions(
        &self,
        dex_network: &str,
    ) -> Result<CurrencyDefinitions, error::CurrencyDefinitions> {
        let &(dex_network, dex_currencies) = &self
            .networks
            .get_id_and_network(dex_network)
            .map(|(id, network)| (id, network.currencies()))
            .ok_or(error::CurrencyDefinitions::NonExistentDexNetwork)?;

        let channels = self.process_channels()?;

        let mut host_currency = None;

        let mut dex_currencies_definitions = vec![];

        dex_currencies_definitions.reserve_exact(dex_currencies.len());

        let host_to_dex_path =
            host_to_dex::find_path(&channels, self.host_network.name(), dex_network)?;

        dex_currencies
            .iter()
            .try_for_each(|(ticker, currency)| {
                match self.resolve_currency(
                    dex_network,
                    &channels,
                    &host_to_dex_path,
                    ticker,
                    currency,
                )? {
                    HostLocality::Local(currency) => {
                        if host_currency.is_none() {
                            host_currency = Some(currency);

                            Ok(())
                        } else {
                            Err(error::CurrencyDefinitions::HostCurrencyAlreadyDefined)
                        }
                    }
                    HostLocality::Remote(currency) => {
                        dex_currencies_definitions.push(currency);

                        Ok(())
                    }
                }
            })
            .and_then(|()| {
                host_currency
                    .map(|host_currency| CurrencyDefinitions {
                        host_currency,
                        dex_currencies: dex_currencies_definitions,
                    })
                    .ok_or(error::CurrencyDefinitions::HostCurrencyNotDefined)
            })
    }

    pub fn network_dexes(&self, network: &str) -> Option<&dex::Dexes> {
        self.networks
            .get_id_and_network(network)
            .map(|(_, network)| network.dexes())
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
}

#[derive(Debug)]
pub struct CurrencyDefinitions {
    pub host_currency: CurrencyDefinition,
    pub dex_currencies: Vec<CurrencyDefinition>,
}
