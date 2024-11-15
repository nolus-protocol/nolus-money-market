use std::collections::BTreeSet;

use serde::Deserialize;

use topology::{CurrencyDefinition, HostCurrency};

use crate::{convert_case, sources::DexCurrencies};

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub(crate) struct Protocol {
    pub dex_network: String,
    pub dex: String,
    pub lpn_ticker: String,
    pub stable_currency_ticker: String,
    pub lease_currencies_tickers: BTreeSet<String>,
    pub payment_only_currencies_tickers: BTreeSet<String>,
}

impl Protocol {
    #[inline]
    pub(super) fn is_protocol_currency(&self, host_currency: &HostCurrency, ticker: &str) -> bool {
        ticker == host_currency.ticker()
            || ticker == self.lpn_ticker
            || self.lease_currencies_tickers.contains(ticker)
            || self.payment_only_currencies_tickers.contains(ticker)
    }

    pub fn dex_currencies<'r>(
        &self,
        host_currency: &HostCurrency,
        dex_currencies: &'r [CurrencyDefinition],
    ) -> DexCurrencies<'r, 'r> {
        dex_currencies
            .iter()
            .filter(|currency_definition| {
                self.is_protocol_currency(host_currency, currency_definition.ticker())
            })
            .map(|currency_definition| {
                (
                    currency_definition.ticker(),
                    (
                        convert_case::snake_case_to_upper_camel_case(currency_definition.ticker()),
                        currency_definition,
                    ),
                )
            })
            .collect()
    }
}
