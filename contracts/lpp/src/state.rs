use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Coin, Decimal, Env, QuerierWrapper, StdResult, Timestamp, Uint128, Uint64};
use cw_storage_plus::{Item, Map};

pub const NANOSECS_IN_YEAR: Uint128 = Uint128::new(365 * 24 * 60 * 60 * 1000 * 1000 * 1000);

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub total_principal_due: Uint128,
    pub total_last_interest: Uint128,
    pub annual_interest_rate: Decimal,
    pub last_update_time: Timestamp,
}

pub fn balance(querier: &QuerierWrapper, env: &Env, config: &Config) -> StdResult<Coin> {
    querier.query_balance(&env.contract.address, &config.denom)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub denom: String,
    pub lease_code_id: Uint64,
    pub base_interest_rate: Decimal,
    pub utilization_optimal: Decimal,
    pub addon_optimal_interest_rate: Decimal,
}

impl Config {
    pub fn new(denom: String, lease_code_id: Uint64) -> Self {
        Config {
            denom,
			lease_code_id,
            base_interest_rate: Decimal::percent(7),
            utilization_optimal: Decimal::percent(70),
            addon_optimal_interest_rate: Decimal::percent(2),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Loan {
	pub principal_due: Uint128,
    // NOTE: currently in percents
	pub annual_interest_rate: Decimal,
	pub interest_paid_by: Timestamp,
}

pub const STATE: Item<State> = Item::new("state");
pub const CONFIG: Item<Config> = Item::new("config");
pub const LOANS: Map<Addr, Loan> = Map::new("loans");
