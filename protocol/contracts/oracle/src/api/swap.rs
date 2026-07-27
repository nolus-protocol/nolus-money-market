use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::result::Result as StdResult;

use currency::{CurrencyDTO, Group};

pub type PoolId = u64;

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
