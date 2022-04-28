use serde::{Serialize, Deserialize};
use schemars::JsonSchema;
use cosmwasm_std::{Uint64, Decimal};

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
