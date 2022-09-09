use std::{
    collections::{HashMap, HashSet},
    convert::TryInto,
    marker::PhantomData,
};

use cosmwasm_std::{Addr, Response, StdError, StdResult, Storage, Timestamp};
use marketprice::market_price::{Parameters, PriceFeeds, PriceFeedsError};

use schemars::JsonSchema;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use std::convert::TryFrom;

use finance::{
    currency::{visit_any, AnyVisitor, Currency, Nls, SymbolOwned},
    duration::Duration,
    price::{Price as FinPrice, PriceDTO},
};

use crate::{state::config::Config, ContractError};

use super::{alarms::MarketAlarms, feeder::Feeders};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Feeds {
    config: Config,
}

impl Feeds {
    const MARKET_PRICE: PriceFeeds<'static> = PriceFeeds::new("market_price");

    pub fn with(config: Config) -> Self {
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

    pub fn get_prices<OracleBase>(
        &self,
        storage: &dyn Storage,
        parameters: Parameters,
        currencies: HashSet<SymbolOwned>,
    ) -> Result<HashMap<SymbolOwned, PriceDTO>, PriceFeedsError>
    where
        OracleBase: Currency,
    {
        let mut prices: HashMap<SymbolOwned, PriceDTO> = HashMap::new();
        for currency in currencies {
            Self::assert_supported_denom(&self.config.supported_denom_pairs, &currency)?;

            let feed = PriceForCurrency::<OracleBase>::cmd(storage, currency.clone(), parameters)?;
            prices.insert(currency, feed);
        }
        Ok(prices)
    }

    fn get_single_price<C, QuoteC>(
        storage: &dyn Storage,
        parameters: Parameters,
    ) -> Result<FinPrice<C, QuoteC>, PriceFeedsError>
    where
        C: Currency,
        QuoteC: Currency,
    {
        Ok(Self::MARKET_PRICE
            .get::<C, QuoteC>(storage, parameters)?
            .try_into()?)
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

pub fn try_feed_prices<OracleBase>(
    storage: &mut dyn Storage,
    block_time: Timestamp,
    sender_raw: Addr,
    prices: Vec<PriceDTO>,
) -> Result<Response, ContractError>
where
    OracleBase: Currency,
{
    // Check feeder permission
    let is_registered = Feeders::is_feeder(storage, &sender_raw)?;
    if !is_registered {
        return Err(ContractError::UnknownFeeder {});
    }

    let config = Config::load(storage)?;
    let oracle = Feeds::with(config.clone());

    // Store the new price feed
    oracle.feed_prices(storage, block_time, &sender_raw, prices)?;

    // Get all currencies registered for alarms
    let hooks_currencies = MarketAlarms::get_hooks_currencies(storage)?;

    let parameters = Feeders::query_config(storage, &config, block_time)?;
    // re-calculate the price of these currencies
    let updated_prices: HashMap<SymbolOwned, PriceDTO> =
        oracle.get_prices::<OracleBase>(storage, parameters, hooks_currencies)?;

    // try notify affected subscribers
    let mut batch = MarketAlarms::try_notify_hooks(storage, updated_prices)?;
    batch.schedule_execute_wasm_reply_error::<_, Nls>(
        &config.timealarms_contract,
        timealarms::msg::ExecuteMsg::Notify(),
        None,
        1,
    )?;
    Ok(Response::from(batch))
}

pub struct PriceForCurrency<'a, QuoteC> {
    storage: &'a dyn Storage,
    parameters: Parameters,
    currency: SymbolOwned,
    _oracle_base: PhantomData<QuoteC>,
}

impl<'a, QuoteC> PriceForCurrency<'a, QuoteC>
where
    QuoteC: Currency,
{
    pub fn cmd(
        storage: &'a dyn Storage,
        currency: SymbolOwned,
        parameters: Parameters,
    ) -> Result<PriceDTO, PriceFeedsError> {
        let visitor = Self {
            storage,
            parameters,
            currency,
            _oracle_base: PhantomData,
        };
        visit_any(&visitor.currency.clone(), visitor)
    }
}

impl<'a, QuoteC> AnyVisitor for PriceForCurrency<'a, QuoteC>
where
    QuoteC: Currency,
{
    type Output = PriceDTO;
    type Error = PriceFeedsError;

    fn on<BaseC>(self) -> Result<Self::Output, Self::Error>
    where
        BaseC: 'static + Currency + DeserializeOwned + Serialize,
    {
        Ok(PriceDTO::try_from(
            Feeds::get_single_price::<BaseC, QuoteC>(self.storage, self.parameters)?,
        )?)
    }
    fn on_unknown(self) -> Result<Self::Output, Self::Error> {
        Err(PriceFeedsError::UnknownCurrency {})
    }
}

#[cfg(test)]
mod tests {
    use crate::contract::feed::Feeds;
    use cosmwasm_std::Addr;
    use finance::{
        coin::Coin,
        currency::{Currency, TestCurrencyA, TestCurrencyB, TestCurrencyC, TestCurrencyD},
        price::{self, PriceDTO},
    };

    use crate::state::config::Config;

    #[test]
    fn test_remove_invalid_prices() {
        let supported_pairs = vec![
            (
                TestCurrencyA::SYMBOL.to_string(),
                TestCurrencyB::SYMBOL.to_string(),
            ),
            (
                TestCurrencyA::SYMBOL.to_string(),
                TestCurrencyC::SYMBOL.to_string(),
            ),
            (
                TestCurrencyB::SYMBOL.to_string(),
                TestCurrencyA::SYMBOL.to_string(),
            ),
            (
                TestCurrencyC::SYMBOL.to_string(),
                TestCurrencyD::SYMBOL.to_string(),
            ),
        ];

        let prices = vec![
            PriceDTO::try_from(price::total_of(Coin::<TestCurrencyB>::new(10)).is(Coin::<
                TestCurrencyA,
            >::new(
                12
            )))
            .unwrap(),
            PriceDTO::try_from(price::total_of(Coin::<TestCurrencyB>::new(10)).is(Coin::<
                TestCurrencyD,
            >::new(
                32
            )))
            .unwrap(),
        ];

        let filtered = Feeds::with(Config::new(
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
                PriceDTO::try_from(price::total_of(Coin::<TestCurrencyB>::new(10)).is(Coin::<
                    TestCurrencyA,
                >::new(
                    12
                )))
                .unwrap(),
            ],
            filtered
        );
    }
}
