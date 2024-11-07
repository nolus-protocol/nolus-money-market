use std::collections::BTreeSet;

use serde::Deserialize;

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
