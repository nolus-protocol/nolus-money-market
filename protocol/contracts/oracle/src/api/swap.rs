use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::result::Result as StdResult;

use currency::{CurrencyDTO, Group};

pub type PoolId = u64;

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "stub_swap_testing"), derive(Debug))]
#[serde(
    deny_unknown_fields,
    rename_all = "snake_case",
    bound(serialize = "", deserialize = "")
)]
pub enum QueryMsg<PriceCurrencies>
where
    PriceCurrencies: Group,
{
    /// Provides a path in the swap tree between two arbitrary currencies
    ///
    /// Returns [`Vec<SwapTarget>`]
    SwapPath {
        from: CurrencyDTO<PriceCurrencies>,
        to: CurrencyDTO<PriceCurrencies>,
    },
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SwapTarget<G>
where
    G: Group,
{
    pub pool_id: PoolId,
    pub target: CurrencyDTO<G>,
}

impl<G> Serialize for SwapTarget<G>
where
    G: Group,
{
    fn serialize<S>(&self, serializer: S) -> StdResult<S::Ok, S::Error>
    where
        S: Serializer,
    {
        (self.pool_id, self.target).serialize(serializer)
    }
}

impl<'de, G> Deserialize<'de> for SwapTarget<G>
where
    G: Group,
{
    fn deserialize<D>(deserializer: D) -> StdResult<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Deserialize::deserialize(deserializer).map(|(pool_id, target)| Self { pool_id, target })
    }
}
