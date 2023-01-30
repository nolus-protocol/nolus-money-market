use serde::{Deserialize, Deserializer, Serialize, Serializer};

use currency::payment::PaymentGroup;
use finance::currency::SymbolOwned;
use schemars::{self, JsonSchema};

pub mod error;
#[cfg(feature = "trx")]
pub mod trx;

pub type PoolId = u64;
pub type SwapGroup = PaymentGroup;

#[derive(Debug, Clone, Eq, PartialEq, JsonSchema)]
#[schemars(with = "(PoolId, SymbolOwned)")]
pub struct SwapTarget {
    pub pool_id: PoolId,
    pub target: SymbolOwned,
}

impl Serialize for SwapTarget {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        (self.pool_id, &self.target).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SwapTarget {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Deserialize::deserialize(deserializer).map(|(pool_id, target)| Self { pool_id, target })
    }
}

pub type SwapPath = Vec<SwapTarget>;
