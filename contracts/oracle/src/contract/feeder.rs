use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use finance::{duration::Duration, fraction::Fraction, percent::Percent};
use marketprice::{feeders::PriceFeeders, market_price::Config as PriceConfig};
use sdk::{
    cosmwasm_ext::Response,
    cosmwasm_std::{Addr, DepsMut, MessageInfo, StdResult, Storage, Timestamp},
};

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

    pub(crate) fn price_config(
        storage: &dyn Storage,
        config: &Config,
        block_time: Timestamp,
    ) -> StdResult<PriceConfig> {
        let registered_feeders = Self::FEEDERS.get(storage)?;
        let all_feeders_cnt = registered_feeders.len();
        let feeders_needed = Self::feeders_needed(all_feeders_cnt, config.expected_feeders);

        Ok(PriceConfig::new(
            config
                .price_feed_period
                .min(Duration::from_nanos(block_time.nanos())),
            feeders_needed,
            block_time,
        ))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use currency::native::Nls;
    use finance::{currency::Currency, percent::Percent};
    use sdk::{
        cosmwasm_ext::Response,
        cosmwasm_std::{
            coins, from_binary,
            testing::{mock_env, mock_info},
            Addr, DepsMut, MessageInfo,
        },
    };

    use crate::{
        contract::{execute, feeder::Feeders, query},
        msg::{ExecuteMsg, QueryMsg},
        tests::{dummy_default_instantiate_msg, setup_test},
        ContractError,
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
    #[should_panic(expected = "Unauthorized")]
    fn register_unauthorized() {
        let (mut deps, _) = setup_test(dummy_default_instantiate_msg());
        let info = mock_info("USER", &coins(1000, Nls::TICKER));

        // register new feeder address
        register(deps.as_mut(), &info, "addr0000").unwrap();
    }

    #[test]
    fn register_feeder() {
        let (mut deps, info) = setup_test(dummy_default_instantiate_msg());

        // register new feeder address
        register(deps.as_mut(), &info, "addr0000").unwrap();

        // check if the new address is added to FEEDERS Item
        let res = query(deps.as_ref(), mock_env(), QueryMsg::Feeders {}).unwrap();
        let resp: HashSet<Addr> = from_binary(&res).unwrap();
        assert_eq!(2, resp.len());
        assert!(resp.contains(&Addr::unchecked("addr0000")));

        // should not add the same address twice
        assert!(register(deps.as_mut(), &info, "addr0000").is_err());

        // register new feeder address
        register(deps.as_mut(), &info, "addr0001").unwrap();
        // check if the new address is added to FEEDERS Item
        let res = query(deps.as_ref(), mock_env(), QueryMsg::Feeders {}).unwrap();
        let resp: HashSet<Addr> = from_binary(&res).unwrap();
        assert_eq!(3, resp.len());
        assert!(resp.contains(&Addr::unchecked("addr0000")));
        assert!(resp.contains(&Addr::unchecked("addr0001")));
    }

    #[test]
    fn remove_feeder() {
        let (mut deps, info) = setup_test(dummy_default_instantiate_msg());

        register(deps.as_mut(), &info, "addr0000").unwrap();
        register(deps.as_mut(), &info, "addr0001").unwrap();
        register(deps.as_mut(), &info, "addr0002").unwrap();
        register(deps.as_mut(), &info, "addr0003").unwrap();

        // check if the new address is added to FEEDERS Item
        let res = query(deps.as_ref(), mock_env(), QueryMsg::Feeders {}).unwrap();
        let resp: HashSet<Addr> = from_binary(&res).unwrap();
        assert_eq!(5, resp.len());
        assert!(resp.contains(&Addr::unchecked("addr0000")));
        assert!(resp.contains(&Addr::unchecked("addr0001")));

        remove(deps.as_mut(), &info, "addr0000");
        remove(deps.as_mut(), &info, "addr0001");
        let res = query(deps.as_ref(), mock_env(), QueryMsg::Feeders {}).unwrap();
        let resp: HashSet<Addr> = from_binary(&res).unwrap();
        assert_eq!(3, resp.len());
        assert!(!resp.contains(&Addr::unchecked("addr0000")));
        assert!(!resp.contains(&Addr::unchecked("addr0001")));
    }

    fn register(
        deps: DepsMut,
        info: &MessageInfo,
        address: &str,
    ) -> Result<Response, ContractError> {
        let msg = ExecuteMsg::RegisterFeeder {
            feeder_address: address.to_string(),
        };
        execute(deps, mock_env(), info.to_owned(), msg)
    }
    fn remove(deps: DepsMut, info: &MessageInfo, address: &str) {
        let msg = ExecuteMsg::RemoveFeeder {
            feeder_address: address.to_string(),
        };
        let _res = execute(deps, mock_env(), info.to_owned(), msg).unwrap();
    }
}
