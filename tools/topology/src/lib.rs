use std::borrow::Borrow;

use serde::Deserialize;

use self::{channels::Channels, currency_resolution::HostLocality, networks::Networks};
pub use self::{currency_definition::CurrencyDefinition, symbol::Symbol};

mod channel;
mod channels;
mod currencies;
pub mod currency;
mod currency_definition;
mod currency_resolution;
mod dex;
pub mod error;
mod host_to_dex;
mod network;
mod networks;
mod symbol;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct Topology {
    host_network: network::Host,
    networks: Networks,
    channels: Channels,
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

        let mut host_currency = None;

        let mut dex_currencies_definitions = vec![];

        dex_currencies_definitions.reserve_exact(dex_currencies.len());

        let host_to_dex_path =
            host_to_dex::find_path(&self.channels, self.host_network.name(), dex_network)?;

        dex_currencies
            .iter()
            .try_for_each(|(ticker, currency)| {
                match self.resolve_currency(
                    dex_network,
                    &self.channels,
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
                    .map(HostCurrency)
                    .map(|host_currency| CurrencyDefinitions {
                        host_currency,
                        dex_currencies: dex_currencies_definitions,
                    })
                    .ok_or(error::CurrencyDefinitions::HostCurrencyNotDefined)
            })
    }
}

#[derive(Debug)]
pub struct CurrencyDefinitions {
    pub host_currency: HostCurrency,
    pub dex_currencies: Vec<CurrencyDefinition>,
}

#[derive(Debug)]
#[repr(transparent)]
pub struct HostCurrency(CurrencyDefinition);

impl HostCurrency {
    pub fn ticker(&self) -> &str {
        self.0.ticker()
    }
}

impl AsRef<CurrencyDefinition> for HostCurrency {
    fn as_ref(&self) -> &CurrencyDefinition {
        &self.0
    }
}

impl Borrow<CurrencyDefinition> for HostCurrency {
    fn borrow(&self) -> &CurrencyDefinition {
        &self.0
    }
}

impl From<HostCurrency> for CurrencyDefinition {
    fn from(HostCurrency(currency_definition): HostCurrency) -> Self {
        currency_definition
    }
}
