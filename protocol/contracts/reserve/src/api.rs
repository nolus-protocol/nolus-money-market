pub(crate) use currencies::Lpns as LpnCurrencies;
pub use currency::SymbolOwned as LpnQueryResponse;
use platform::contract::{Code, CodeId};
use serde::{Deserialize, Serialize};

use finance::coin::CoinDTO;
use sdk::{
    cosmwasm_std::Uint64,
    schemars::{self, JsonSchema},
};

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
    // This is an internal system API and we use [Code]
    NewLeaseCode(Code),

    CoverLiquidationLosses(LpnCoin),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum SudoMsg {}

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq, JsonSchema)]
#[cfg_attr(feature = "testing", derive(Debug))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum QueryMsg {
    /// Return a [LpnQueryResponse] of the Lpn this reserve holds
    ReserveLpn(), // the name contains the contract name to help distinguish from simmilar queries to other contracts
    /// Return a [ConfigResponse]
    Config(),
}

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq, JsonSchema)]
#[cfg_attr(feature = "testing", derive(Debug))]
pub struct ConfigResponse {
    lease_code_id: Uint64,
}

impl ConfigResponse {
    pub fn new(lease: Code) -> Self {
        Self {
            lease_code_id: CodeId::from(lease).into(),
        }
    }
}
