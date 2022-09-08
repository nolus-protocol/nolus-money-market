use std::collections::{HashMap, HashSet};

use cosmwasm_std::{Addr, StdError, StdResult, Storage, Timestamp};
use marketprice::market_price::{PriceFeeds, PriceFeedsError};

use schemars::JsonSchema;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use std::convert::TryFrom;

use finance::{
    currency::{visit_any, AnyVisitor, Currency, SymbolOwned, Usdc},
    duration::Duration,
    price::{Price as FinPrice, PriceDTO},
};

use crate::{state::config::Config, ContractError};

use super::feeder::Feeders;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Feeds {
    config: Config,
}

impl Feeds {
    const MARKET_PRICE: PriceFeeds<'static> = PriceFeeds::new("market_price");

    pub fn new(config: Config) -> Self {
        Self { config }
    }

    fn assert_supported_denom(
        supported_denom_pairs: &[(SymbolOwned, SymbolOwned)],
        currency: &SymbolOwned,
    ) -> StdResult<()> {
        let mut all_supported_denoms = HashSet::<SymbolOwned>::new();
        for pair in supported_denom_pairs {
            all_supported_denoms.insert(pair.0.clone());
            all_supported_denoms.insert(pair.1.clone());
        }
        if !all_supported_denoms.contains(currency) {
            return Err(StdError::generic_err("Unsupported denom"));
        }
        Ok(())
    }

    pub fn get_prices(
        &self,
        storage: &dyn Storage,
        block_time: Timestamp,
        currencies: HashSet<SymbolOwned>,
    ) -> Result<HashMap<SymbolOwned, PriceDTO>, PriceFeedsError> {
        let mut prices: HashMap<SymbolOwned, PriceDTO> = HashMap::new();
        for currency in currencies {
            Self::assert_supported_denom(&self.config.supported_denom_pairs, &currency)?;

            let feed = QueryWithLpn::cmd(storage, block_time, currency.clone())?;
            prices.insert(currency, feed);
        }
        Ok(prices)
    }

    pub fn get_single_price<C, QuoteC>(
        storage: &dyn Storage,
        block_time: Timestamp,
    ) -> Result<FinPrice<C, QuoteC>, PriceFeedsError>
    where
        C: Currency,
        QuoteC: Currency,
    {
        let config = Config::load(storage)?;

        let price_query = Feeders::query_config(storage, &config)?;
        let price = Self::MARKET_PRICE.get_converted_price(storage, block_time, price_query)?;

        Ok(price)
    }

    pub fn feed_prices(
        &self,
        storage: &mut dyn Storage,
        block_time: Timestamp,
        sender_raw: &Addr,
        prices: Vec<PriceDTO>,
    ) -> Result<(), ContractError> {
        // FIXME: refactore this once the supported pairs refactoring is done
        let filtered_prices = self.remove_invalid_prices(prices);
        if filtered_prices.is_empty() {
            return Err(ContractError::UnsupportedDenomPairs {});
        }

        Self::MARKET_PRICE.feed(
            storage,
            block_time,
            sender_raw,
            filtered_prices,
            Duration::from_secs(self.config.price_feed_period_secs),
        )?;

        Ok(())
    }

    fn remove_invalid_prices(&self, prices: Vec<PriceDTO>) -> Vec<PriceDTO> {
        prices
            .iter()
            .filter(|price| {
                self.config.supported_denom_pairs.contains(&(
                    price.base().symbol().to_string(),
                    price.quote().symbol().to_string(),
                )) && !price
                    .base()
                    .symbol()
                    .eq_ignore_ascii_case(&price.quote().symbol())
            })
            .map(|p| p.to_owned())
            .collect()
    }
}

struct QueryWithLpn<'a> {
    storage: &'a dyn Storage,
    block_time: Timestamp,
    currency: SymbolOwned,
}

impl<'a> QueryWithLpn<'a> {
    pub fn cmd(
        storage: &'a dyn Storage,
        block_time: Timestamp,
        currency: SymbolOwned,
    ) -> Result<PriceDTO, PriceFeedsError> {
        let visitor = Self {
            storage,
            block_time,
            currency,
        };
        visit_any(&visitor.currency.clone(), visitor)
    }
}

impl<'a> AnyVisitor for QueryWithLpn<'a> {
    type Output = PriceDTO;
    type Error = PriceFeedsError;

    fn on<LPN>(self) -> Result<Self::Output, Self::Error>
    where
        LPN: 'static + Currency + DeserializeOwned + Serialize,
    {
        Ok(PriceDTO::try_from(Feeds::get_single_price::<LPN, Usdc>(
            self.storage,
            self.block_time,
        )?)?)
    }
    fn on_unknown(self) -> Result<Self::Output, Self::Error> {
        Err(PriceFeedsError::UnknownCurrency {})
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        contract::feed::Feeds,
        tests::common::{A, B, D},
    };
    use cosmwasm_std::Addr;
    use finance::{
        coin::Coin,
        price::{self, PriceDTO},
    };

    use crate::state::config::Config;

    #[test]
    fn test_remove_invalid_prices() {
        let supported_pairs = vec![
            ("A".to_string(), "B".to_string()),
            ("A".to_string(), "C".to_string()),
            ("B".to_string(), "A".to_string()),
            ("C".to_string(), "D".to_string()),
        ];

        let prices = vec![
            PriceDTO::try_from(price::total_of(Coin::<B>::new(10)).is(Coin::<A>::new(12))).unwrap(),
            PriceDTO::try_from(price::total_of(Coin::<B>::new(10)).is(Coin::<D>::new(32))).unwrap(),
        ];

        let filtered = Feeds::new(Config::new(
            "denom".to_string(),
            Addr::unchecked("owner"),
            20,
            5,
            supported_pairs,
            Addr::unchecked("timealarms_contract"),
        ))
        .remove_invalid_prices(prices);

        assert_eq!(
            vec![
                PriceDTO::try_from(price::total_of(Coin::<B>::new(10)).is(Coin::<A>::new(12)))
                    .unwrap(),
            ],
            filtered
        );
    }
}
