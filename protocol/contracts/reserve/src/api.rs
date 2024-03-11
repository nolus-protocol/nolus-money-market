pub(crate) use currencies::{Lpn as LpnCurrency, Lpns as LpnCurrencies};
use currency::{Currency, SymbolOwned};
use serde::{Deserialize, Serialize};

use finance::coin::CoinDTO;
use sdk::{
    cosmwasm_std::Uint64,
    schemars::{self, JsonSchema},
};

use crate::Config;

pub(crate) type LpnCoin = CoinDTO<LpnCurrencies>;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct InstantiateMsg {
    /// Unchecked address of the user that can change the lease code Id
    pub lease_code_admin: String,
    pub lease_code_id: Uint64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct MigrateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum ExecuteMsg {
    NewLeaseCode { code_id: Uint64 },

    CoverLiquidationLosses { amount: LpnCoin },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum SudoMsg {}

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq, JsonSchema)]
#[cfg_attr(feature = "testing", derive(Debug))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum QueryMsg {
    /// Return a [ConfigResponse]
    Config(),
}

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub lpn_ticker: SymbolOwned,
    pub lease_code_id: Uint64,
}

impl From<Config> for ConfigResponse {
    fn from(cfg: Config) -> Self {
        Self {
            lpn_ticker: LpnCurrency::TICKER.into(),
            lease_code_id: cfg.lease_code_id().into(),
        }
    }
}
