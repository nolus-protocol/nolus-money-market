use std::{collections::HashSet, convert::TryInto};

use cosmwasm_std::{Addr, DepsMut, StdError, StdResult, Storage, Timestamp};
use marketprice::{
    feed::{Denom, DenomToPrice, Price},
    feeders::{PriceFeeders, PriceFeedersError},
    hooks::price::PriceHooks,
    market_price::{PriceFeeds, PriceQuery},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use std::convert::TryFrom;

use crate::{alarms::MarketAlarms, state::config::Config, ContractError};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MarketOracle {}

impl MarketOracle {
    const FEEDERS: PriceFeeders<'static> = PriceFeeders::new("feeders");
    const MARKET_PRICE: PriceFeeds<'static> = PriceFeeds::new("market_price");

    pub fn get_feeders(storage: &dyn Storage) -> StdResult<HashSet<Addr>> {
        Self::FEEDERS.get(storage)
    }

    pub fn is_feeder(storage: &dyn Storage, address: &Addr) -> StdResult<bool> {
        Self::FEEDERS.is_registered(storage, address)
    }

    pub fn register_feeder(deps: DepsMut, address: Addr) -> Result<(), PriceFeedersError> {
        Self::FEEDERS.register(deps, address)
    }

    fn init_price_query(
        storage: &dyn Storage,
        base: Denom,
        config: &Config,
    ) -> StdResult<PriceQuery> {
        let price_feed_period = config.price_feed_period;

        Self::assert_supported_denom(&config.supported_denom_pairs, base.clone())?;

        let registered_feeders = Self::FEEDERS.get(storage)?;
        let all_feeders_cnt = registered_feeders.len();
        let feeders_needed =
            Self::feeders_needed(all_feeders_cnt, config.feeders_percentage_needed);

        Ok(PriceQuery::new(
            (base, config.base_asset.clone()),
            price_feed_period,
            feeders_needed,
        ))
    }

    fn assert_supported_denom(
        supported_denom_pairs: &[(Denom, Denom)],
        denom: Denom,
    ) -> StdResult<()> {
        let mut all_supported_denoms = HashSet::<Denom>::new();
        for pair in supported_denom_pairs {
            all_supported_denoms.insert(pair.0.clone());
            all_supported_denoms.insert(pair.1.clone());
        }
        if !all_supported_denoms.contains(&denom) {
            return Err(StdError::generic_err("Unsupported denom"));
        }
        Ok(())
    }

    pub fn get_price_for(
        storage: &dyn Storage,
        block_time: Timestamp,
        denoms: Vec<Denom>,
    ) -> StdResult<Vec<DenomToPrice>> {
        let config = Config::load(storage)?;
        let mut prices: Vec<DenomToPrice> = Vec::new();
        for denom in denoms {
            let price_query = Self::init_price_query(storage, denom.clone(), &config)?;
            let resp = Self::MARKET_PRICE.get(storage, block_time, price_query);
            match resp {
                Ok(feed) => {
                    prices.push(DenomToPrice::new(
                        denom,
                        Price::new(feed, config.base_asset.clone()),
                    ));
                }
                Err(err) => return Err(StdError::generic_err(err.to_string())),
            };
        }
        Ok(prices)
    }

    pub fn feed_prices(
        storage: &mut dyn Storage,
        block_time: Timestamp,
        sender_raw: &Addr,
        base: &Denom,
        prices: Vec<Price>,
    ) -> Result<(), ContractError> {
        let config = Config::load(storage)?;

        let filtered_prices =
            Self::remove_invalid_prices(config.supported_denom_pairs, base.clone(), prices);
        if filtered_prices.is_empty() {
            return Err(ContractError::UnsupportedDenomPairs {});
        }

        Self::MARKET_PRICE.feed(
            storage,
            block_time,
            sender_raw,
            base.to_string(),
            filtered_prices,
            config.price_feed_period,
        )?;

        Ok(())
    }

    // this is a helper function so Decimal works with u64 rather than Uint128
    // also, we must *round up* here, as we need 8, not 7 feeders to reach 50% of 15 total
    fn feeders_needed(weight: usize, percentage: u8) -> usize {
        let weight128 = u128::try_from(weight).expect("usize to u128 overflow");
        let res = weight128 * u128::from(percentage) / 100;
        res.try_into().expect("usize overflow")
    }

    fn remove_invalid_prices(
        supported_denom_pairs: Vec<(String, String)>,
        base: Denom,
        prices: Vec<Price>,
    ) -> Vec<Price> {
        prices
            .iter()
            .filter(|price| {
                supported_denom_pairs.contains(&(base.clone(), price.denom.clone()))
                    && !base.eq_ignore_ascii_case(&price.denom)
            })
            .map(|p| p.to_owned())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use cosmwasm_std::Decimal;
    use marketprice::feed::Price;

    use crate::oracle::MarketOracle;

    #[test]
    // we ensure this rounds up (as it calculates needed votes)
    fn feeders_needed_rounds_properly() {
        // round up right below 1
        assert_eq!(7, MarketOracle::feeders_needed(3, 255));
        // round up right over 1
        assert_eq!(7, MarketOracle::feeders_needed(3, 254));
        assert_eq!(76, MarketOracle::feeders_needed(30, 254));

        // exact matches don't round
        assert_eq!(17, MarketOracle::feeders_needed(34, 50));
        assert_eq!(12, MarketOracle::feeders_needed(48, 25));
    }

    #[test]
    fn test_remove_invalid_prices() {
        let supported_pairs = vec![
            ("A".to_string(), "B".to_string()),
            ("A".to_string(), "C".to_string()),
            ("B".to_string(), "A".to_string()),
            ("C".to_string(), "D".to_string()),
        ];

        let filtered = MarketOracle::remove_invalid_prices(
            supported_pairs,
            "B".to_string(),
            vec![
                Price {
                    denom: "A".to_string(),
                    amount: Decimal::from_str("1.2").unwrap(),
                },
                Price {
                    denom: "D".to_string(),
                    amount: Decimal::from_str("3.2").unwrap(),
                },
                Price {
                    denom: "B".to_string(),
                    amount: Decimal::from_str("1.2").unwrap(),
                },
            ],
        );

        assert_eq!(
            vec![Price {
                denom: "A".to_string(),
                amount: Decimal::from_str("1.2").unwrap()
            }],
            filtered
        );
    }
}
