use std::{collections::HashSet, convert::TryInto};

use cosmwasm_std::{Addr, DepsMut, MessageInfo, Response, StdResult, Storage};
use finance::duration::Duration;
use marketprice::{feeders::PriceFeeders, market_price::QueryConfig};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use std::convert::TryFrom;

use crate::{state::config::Config, ContractError};
const PRECISION_FACTOR: u128 = 1_000_000_000;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Feeders {
    config: Config,
}

impl Feeders {
    const FEEDERS: PriceFeeders<'static> = PriceFeeders::new("feeders");

    pub fn get(storage: &dyn Storage) -> StdResult<HashSet<Addr>> {
        Self::FEEDERS.get(storage)
    }

    pub fn is_feeder(storage: &dyn Storage, address: &Addr) -> StdResult<bool> {
        Self::FEEDERS.is_registered(storage, address)
    }

    pub fn try_register(
        deps: DepsMut,
        info: MessageInfo,
        address: String,
    ) -> Result<Response, ContractError> {
        let config = Config::load(deps.storage)?;
        if info.sender != config.owner {
            return Err(ContractError::Unauthorized {});
        }
        // check if address is valid
        let f_address = deps.api.addr_validate(&address)?;
        Self::FEEDERS.register(deps, f_address)?;

        Ok(Response::new())
    }

    // this is a helper function so Decimal works with u64 rather than Uint128
    // also, we must *round up* here, as we need 8, not 7 feeders to reach 50% of 15 total
    fn feeders_needed(weight: usize, percentage: u8) -> usize {
        let weight128 = u128::try_from(weight).expect("usize to u128 overflow");
        let res = (PRECISION_FACTOR * weight128) * u128::from(percentage) / 100;
        ((res + PRECISION_FACTOR - 1) / PRECISION_FACTOR)
            .try_into()
            .expect("usize overflow")
    }

    pub fn query_config(storage: &dyn Storage, config: &Config) -> StdResult<QueryConfig> {
        let registered_feeders = Self::FEEDERS.get(storage)?;
        let all_feeders_cnt = registered_feeders.len();
        let feeders_needed =
            Self::feeders_needed(all_feeders_cnt, config.feeders_percentage_needed);

        Ok(QueryConfig::new(
            Duration::from_secs(config.price_feed_period_secs),
            feeders_needed,
        ))
    }
}

#[cfg(test)]
mod tests {

    use crate::contract::feeder::Feeders;

    #[test]
    // we ensure this rounds up (as it calculates needed votes)
    fn feeders_needed_rounds_properly() {
        // round up right below 1
        assert_eq!(8, Feeders::feeders_needed(3, 255));
        // round up right over 1
        assert_eq!(8, Feeders::feeders_needed(3, 254));
        assert_eq!(77, Feeders::feeders_needed(30, 254));

        // exact matches don't round
        assert_eq!(17, Feeders::feeders_needed(34, 50));
        assert_eq!(12, Feeders::feeders_needed(48, 25));
        assert_eq!(2, Feeders::feeders_needed(132, 1));
        assert_eq!(2, Feeders::feeders_needed(189, 1));
    }
}
