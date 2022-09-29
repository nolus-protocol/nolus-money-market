use std::collections::HashSet;

use cosmwasm_std::{Addr, DepsMut, MessageInfo, Response, StdResult, Storage, Timestamp};
use serde::{Deserialize, Serialize};

use finance::{fraction::Fraction, percent::Percent};
use marketprice::{feeders::PriceFeeders, market_price::Parameters};

use crate::{state::Config, ContractError};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Feeders {
    config: Config,
}

impl Feeders {
    const FEEDERS: PriceFeeders<'static> = PriceFeeders::new("feeders");
    const PRECISION_FACTOR: u128 = 1_000_000_000;

    pub(crate) fn get(storage: &dyn Storage) -> StdResult<HashSet<Addr>> {
        Self::FEEDERS.get(storage)
    }

    pub(crate) fn is_feeder(storage: &dyn Storage, address: &Addr) -> StdResult<bool> {
        Self::FEEDERS.is_registered(storage, address)
    }

    pub(crate) fn try_register(
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

    pub(crate) fn try_remove(
        deps: DepsMut,
        info: MessageInfo,
        address: String,
    ) -> Result<Response, ContractError> {
        let f_address = deps.api.addr_validate(&address)?;
        if !Self::is_feeder(deps.storage, &f_address)? {
            return Err(ContractError::UnknownFeeder {});
        }

        let config = Config::load(deps.storage)?;
        if info.sender != config.owner {
            return Err(ContractError::Unauthorized {});
        }

        Self::FEEDERS.remove(deps, f_address)?;
        Ok(Response::default())
    }

    // this is a helper function so Decimal works with u64 rather than Uint128
    // also, we must *round up* here, as we need 8, not 7 feeders to reach 50% of 15 total
    fn feeders_needed(weight: usize, percentage: Percent) -> usize {
        let weight128 = u128::try_from(weight).expect("usize to u128 overflow");

        let res = percentage.of(Self::PRECISION_FACTOR * weight128);
        ((res + Self::PRECISION_FACTOR - 1) / Self::PRECISION_FACTOR)
            .try_into()
            .expect("usize overflow")
    }

    pub(crate) fn query_config(
        storage: &dyn Storage,
        config: &Config,
        block_time: Timestamp,
    ) -> StdResult<Parameters> {
        let registered_feeders = Self::FEEDERS.get(storage)?;
        let all_feeders_cnt = registered_feeders.len();
        let feeders_needed =
            Self::feeders_needed(all_feeders_cnt, config.feeders_percentage_needed);

        Ok(Parameters::new(
            config.price_feed_period,
            feeders_needed,
            block_time,
        ))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use cosmwasm_std::{from_binary, testing::mock_env, Addr};

    use finance::percent::Percent;

    use crate::{
        contract::{execute, feeder::Feeders, query},
        msg::{ExecuteMsg, QueryMsg},
        tests::{dummy_default_instantiate_msg, setup_test},
    };

    #[test]
    // we ensure this rounds up (as it calculates needed votes)
    fn feeders_needed_rounds_properly() {
        // round up right below 1
        assert_eq!(8, Feeders::feeders_needed(3, Percent::from_percent(255)));
        // round up right over 1
        assert_eq!(8, Feeders::feeders_needed(3, Percent::from_percent(254)));
        assert_eq!(77, Feeders::feeders_needed(30, Percent::from_percent(254)));

        // exact matches don't round
        assert_eq!(17, Feeders::feeders_needed(34, Percent::from_percent(50)));
        assert_eq!(12, Feeders::feeders_needed(48, Percent::from_percent(25)));
        assert_eq!(2, Feeders::feeders_needed(132, Percent::from_percent(1)));
        assert_eq!(2, Feeders::feeders_needed(189, Percent::from_percent(1)));
    }

    #[test]
    fn register_feeder() {
        let (mut deps, info) = setup_test(dummy_default_instantiate_msg());

        // register new feeder address
        let msg = ExecuteMsg::RegisterFeeder {
            feeder_address: "addr0000".to_string(),
        };
        let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        // check if the new address is added to FEEDERS Item
        let res = query(deps.as_ref(), mock_env(), QueryMsg::Feeders {}).unwrap();
        let resp: HashSet<Addr> = from_binary(&res).unwrap();
        assert_eq!(2, resp.len());
        assert!(resp.contains(&Addr::unchecked("addr0000")));

        // should not add the same address twice
        let msg = ExecuteMsg::RegisterFeeder {
            feeder_address: "addr0000".to_string(),
        };
        let _ = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
        // validate that the address in not added twice
        let res = query(deps.as_ref(), mock_env(), QueryMsg::Feeders {}).unwrap();
        let resp: HashSet<Addr> = from_binary(&res).unwrap();
        assert_eq!(2, resp.len());

        // register new feeder address
        let msg = ExecuteMsg::RegisterFeeder {
            feeder_address: "addr0001".to_string(),
        };
        let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        // check if the new address is added to FEEDERS Item
        let res = query(deps.as_ref(), mock_env(), QueryMsg::Feeders {}).unwrap();
        let resp: HashSet<Addr> = from_binary(&res).unwrap();
        assert_eq!(3, resp.len());
        assert!(resp.contains(&Addr::unchecked("addr0000")));
        assert!(resp.contains(&Addr::unchecked("addr0001")));
    }
}
