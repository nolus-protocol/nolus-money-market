use serde::{Deserialize, Serialize};

use currency::native::Nls;
use finance::coin::Coin;
use sdk::{
    cosmwasm_std::Addr,
    schemars::{self, JsonSchema},
};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub enum MigrateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct InstantiateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    ConfigureRewardTransfer { rewards_dispatcher: Addr },
    SendRewards { amount: Coin<Nls> },
}
