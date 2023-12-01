use serde::{Deserialize, Serialize};

use sdk::{
    cosmwasm_std::Addr,
    schemars::{self, JsonSchema},
};

#[cfg(feature = "contract")]
mod impl_mod;

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Protocol<T> {
    pub leaser: T,
    pub lpp: T,
    pub oracle: T,
    pub profit: T,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ProtocolWithNetworkName {
    pub network: String,
    pub protocol: Protocol<Addr>,
}
