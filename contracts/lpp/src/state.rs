use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Coin, Decimal, Timestamp, QuerierWrapper, StdResult, Env};
use cw_storage_plus::Item;

pub const NANOSECS_IN_YEAR: u64 = 365*24*60*60*1000*1000*1000;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub total_principal_due: Coin,
    pub total_last_interest: Coin,
	pub annual_interest_rate: Decimal,
	pub last_update_time: Timestamp,
}

pub fn balance(querier: &QuerierWrapper, env: &Env, config: &Config) -> StdResult<Coin> {
	querier.query_balance(&env.contract.address, &config.denom)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub denom: String,
    pub base_interest_rate: Decimal,
    pub utilization_optimal: Decimal,
    pub addon_optimal_interest_rate: Decimal,
}

impl Config {
	pub fn new(denom: &str) -> Self {
		Config {
            denom: denom.into(),
            base_interest_rate: Decimal::percent(7),
            utilization_optimal: Decimal::percent(70),
            addon_optimal_interest_rate: Decimal::percent(2),
		}
	}
}

pub const STATE: Item<State> = Item::new("state");
pub const CONFIG: Item<Config> = Item::new("config");

