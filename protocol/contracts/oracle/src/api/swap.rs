use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::result::Result as StdResult;

use thiserror::Error;

use currency::SymbolOwned;
use sdk::{
    cosmwasm_std::StdError,
    schemars::{self, JsonSchema},
};

pub type PoolId = u64;
pub type Result<T> = StdResult<T, Error>;

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum QueryMsg {
    /// Provides a path in the swap tree between two arbitrary currencies
    ///
    /// Returns `self::SwapPath`
    SwapPath { from: SymbolOwned, to: SymbolOwned },
}

pub type SwapPath = Vec<SwapTarget>;

#[derive(Error, Debug, PartialEq)]
pub enum Error {
    #[error("[Oracle; Stub] Failed to query swap path! Cause: {0}")]
    StubSwapPathQuery(StdError),
}

#[derive(Debug, Clone, Eq, PartialEq, JsonSchema)]
#[schemars(with = "(PoolId, SymbolOwned)")]
pub struct SwapTarget {
    pub pool_id: PoolId,
    pub target: SymbolOwned,
}

impl Serialize for SwapTarget {
    fn serialize<S>(&self, serializer: S) -> StdResult<S::Ok, S::Error>
    where
        S: Serializer,
    {
        (self.pool_id, &self.target).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SwapTarget {
    fn deserialize<D>(deserializer: D) -> StdResult<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Deserialize::deserialize(deserializer).map(|(pool_id, target)| Self { pool_id, target })
    }
}
