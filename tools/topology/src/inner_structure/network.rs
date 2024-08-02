use std::collections::BTreeMap;

use serde::Deserialize;

use super::Currency;

#[derive(Debug, Clone, Deserialize)]
#[serde(from = "self::NetworkRaw")]
pub(crate) struct Network {
    pub currencies: BTreeMap<String, Currency>,
}

impl From<NetworkRaw> for Network {
    fn from(NetworkRaw { currencies, .. }: NetworkRaw) -> Self {
        Self { currencies }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
struct NetworkRaw {
    currencies: BTreeMap<String, Currency>,
    #[serde(rename = "amm_pools")]
    _amm_pools: Option<Vec<AmmPoolRaw>>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
struct AmmPoolRaw {
    #[serde(rename = "id")]
    _id: String,
    #[serde(rename = "token_0")]
    _token_0: String,
    #[serde(rename = "token_1")]
    _token_1: String,
}
