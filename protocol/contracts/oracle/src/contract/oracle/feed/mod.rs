use std::marker::PhantomData;

use currency::{CurrencyDTO, CurrencyDef, Group, MemberOf};
use finance::price::{base::BasePrice, dto::PriceDTO};
use marketprice::{config::Config, market_price::PriceFeeds};
use sdk::cosmwasm_std::{Addr, Storage, Timestamp};

use crate::{
    api::{swap::SwapTarget, SwapLeg},
    error::{self, ContractError},
    state::supported_pairs::SupportedPairs,
};

use self::{leg_cmd::LegCmd, price_querier::FedPrices};

use super::PriceResult;

mod leg_cmd;
mod price_querier;

pub struct Feeds<PriceG, BaseC, BaseG> {
    feeds: PriceFeeds<PriceG>,
    _base_c: PhantomData<BaseC>,
    _base_g: PhantomData<BaseG>,
}

impl<PriceG, BaseC, BaseG> Feeds<PriceG, BaseC, BaseG>
where
    PriceG: Group<TopG = PriceG>,
    BaseC: CurrencyDef,
    BaseC::Group: MemberOf<BaseG> + MemberOf<PriceG>,
    BaseG: Group + MemberOf<PriceG>,
{
    pub(crate) fn with(config: Config) -> Self {
        Self {
            feeds: PriceFeeds::new("market_price", config),
            _base_c: PhantomData,
            _base_g: PhantomData,
        }
    }

    pub(crate) fn feed_prices(
        &self,
        storage: &mut dyn Storage,
        tree: &SupportedPairs<PriceG, BaseC>,
        block_time: Timestamp,
        sender_raw: Addr,
        prices: &[PriceDTO<PriceG>],
    ) -> Result<(), ContractError> {
        if let Some(unsupported) = prices.iter().find(|price| {
            !tree.swap_pairs_df().any(
                |SwapLeg {
                     from,
                     to: SwapTarget { target: to, .. },
                 }| {
                    price
                        .base()
                        .of_currency_dto(&from)
                        .and_then(|()| price.quote().of_currency_dto(&to))
                        .is_ok()
                },
            )
        }) {
            Err(error::unsupported_denom_pairs(unsupported))
        } else {
            self.feeds
                .feed(storage, block_time, sender_raw, prices)
                .map_err(Into::into)
        }
    }

    pub fn all_prices_iter<'r, 'self_, 'storage, I>(
        &'self_ self,
        storage: &'storage dyn Storage,
        swap_pairs_df: I,
        at: Timestamp,
        total_feeders: usize,
    ) -> impl Iterator<Item = PriceResult<PriceG, BaseC, BaseG>> + 'r
    where
        'self_: 'r,
        'storage: 'r,
        I: Iterator<Item = SwapLeg<PriceG>> + 'r,
    {
        let cmd: LegCmd<PriceG, BaseC, BaseG, FedPrices<'_, PriceG>> =
            LegCmd::new(FedPrices::new(storage, &self.feeds, at, total_feeders));

        swap_pairs_df
            .scan(cmd, |cmd, leg: SwapLeg<PriceG>| {
                Some(currency::visit_any_on_currencies(leg.from, leg.to.target, cmd).transpose())
            })
            .flatten()
    }

    pub fn calc_base_price(
        &self,
        storage: &dyn Storage,
        tree: &SupportedPairs<PriceG, BaseC>,
        currency: &CurrencyDTO<PriceG>,
        at: Timestamp,
        total_feeders: usize,
    ) -> Result<BasePrice<PriceG, BaseC, BaseG>, ContractError> {
        let dto = self
            .feeds
            .price::<BaseC, _, _>(
                currency::dto::<BaseC, _>(),
                storage,
                at,
                total_feeders,
                tree.load_path(currency)?,
            )
            .map_err(Into::<ContractError>::into)?;
        Ok(dto)
        // BasePrice::from_dto_price(dto, &currency::dto::<BaseC, _>()).map_err(Into::into)
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use currencies::{Lpn as BaseCurrency, PaymentGroup as PriceCurrencies};
    use currency::{Currency, CurrencyDTO, CurrencyDef, MemberOf, SymbolStatic};
    use finance::{
        coin::Amount,
        price::{dto::PriceDTO, Price},
    };
    use marketprice::alarms::prefix::Prefix;

    use crate::{tests, ContractError};

    use super::price_querier::PriceQuerier;

    #[derive(Clone)]
    pub struct TestFeeds(pub HashMap<(SymbolStatic, SymbolStatic), PriceDTO<PriceCurrencies>>);
    impl TestFeeds {
        pub fn add<B, Q>(&mut self, total_of: Amount, is: Amount)
        where
            B: CurrencyDef,
            B::Group: MemberOf<PriceCurrencies>,
            Q: CurrencyDef,
            Q::Group: MemberOf<PriceCurrencies>,
        {
            self.0.insert(
                (B::ticker(), Q::ticker()),
                tests::dto_price::<B, PriceCurrencies, Q>(total_of, is),
            );
        }
    }

    impl PriceQuerier for TestFeeds {
        type CurrencyGroup = PriceCurrencies;

        fn price<C, QuoteC>(
            &self,
            amount_c: &CurrencyDTO<Self::CurrencyGroup>,
            quote_c: &CurrencyDTO<Self::CurrencyGroup>,
        ) -> Result<Option<Price<C, QuoteC>>, ContractError>
        where
            C: Currency + MemberOf<Self::CurrencyGroup>,
            QuoteC: Currency + MemberOf<Self::CurrencyGroup>,
        {
            Ok(self
                .0
                .get(&(amount_c.first_key(), quote_c.first_key()))
                .map(|dto| dto.as_specific(amount_c, quote_c)))
        }
    }

    mod all_prices_iter {
        use currencies::{
            Lpns as BaseCurrencies, PaymentC1, PaymentC3, PaymentC4, PaymentC5, PaymentC6,
            PaymentC7, PaymentGroup as PriceCurrencies,
        };
        use finance::{duration::Duration, percent::Percent, price::base::BasePrice};
        use marketprice::config::Config;
        use sdk::cosmwasm_std::{
            testing::{self, MockStorage},
            Addr,
        };

        use super::BaseCurrency;
        use crate::{
            contract::oracle::feed::Feeds, state::supported_pairs::SupportedPairs, test_tree, tests,
        };

        #[test]
        fn normal() {
            let mut storage = MockStorage::new();
            let env = testing::mock_env();
            let tree = test_tree::dummy_swap_tree();
            let tree = SupportedPairs::<PriceCurrencies, BaseCurrency>::new::<BaseCurrency>(
                tree.into_tree(),
            )
            .unwrap();
            tree.save(&mut storage).unwrap();

            let config = Config::new(
                Percent::HUNDRED,
                Duration::from_secs(5),
                10,
                Percent::from_percent(50),
            );

            let oracle: Feeds<PriceCurrencies, BaseCurrency, BaseCurrencies> = Feeds::with(config);

            oracle
                .feed_prices(
                    &mut storage,
                    &tree,
                    env.block.time,
                    Addr::unchecked("feeder"),
                    &[
                        tests::dto_price::<PaymentC4, _, BaseCurrency>(2, 1),
                        tests::dto_price::<PaymentC1, _, BaseCurrency>(5, 1),
                        tests::dto_price::<PaymentC7, _, PaymentC1>(3, 1),
                        tests::dto_price::<PaymentC5, _, PaymentC4>(7, 1),
                        tests::dto_price::<PaymentC6, _, PaymentC4>(3, 1),
                        tests::dto_price::<PaymentC3, _, PaymentC5>(11, 1),
                    ],
                )
                .unwrap();

            let prices: Vec<_> = oracle
                .all_prices_iter(&storage, tree.swap_pairs_df(), env.block.time, 1)
                .flatten()
                .collect();

            let expected: Vec<BasePrice<PriceCurrencies, BaseCurrency, BaseCurrencies>> = vec![
                tests::base_price::<PaymentC4>(2, 1),
                tests::base_price::<PaymentC5>(2 * 7, 1),
                tests::base_price::<PaymentC3>(2 * 7 * 11, 1),
                tests::base_price::<PaymentC6>(6, 1),
                tests::base_price::<PaymentC1>(5, 1),
                tests::base_price::<PaymentC7>(3 * 5, 1),
            ];

            assert_eq!(expected, prices);
        }

        #[test]
        fn missing_price() {
            let mut storage = MockStorage::new();
            let env = testing::mock_env();
            let tree = test_tree::dummy_swap_tree();
            let tree = SupportedPairs::<PriceCurrencies, BaseCurrency>::new::<BaseCurrency>(
                tree.into_tree(),
            )
            .unwrap();
            tree.save(&mut storage).unwrap();

            let config = Config::new(
                Percent::HUNDRED,
                Duration::from_secs(5),
                10,
                Percent::from_percent(50),
            );

            let oracle: Feeds<PriceCurrencies, BaseCurrency, BaseCurrencies> = Feeds::with(config);

            oracle
                .feed_prices(
                    &mut storage,
                    &tree,
                    env.block.time,
                    Addr::unchecked("feeder"),
                    &[
                        // tests::dto_price::<PaymentC1, _, BaseCurrency, _>(5, 1), a gap for PaymentC7
                        tests::dto_price::<PaymentC4, _, BaseCurrency>(2, 1),
                        tests::dto_price::<PaymentC7, _, PaymentC1>(10, 1),
                        tests::dto_price::<PaymentC5, _, PaymentC4>(1, 1),
                        tests::dto_price::<PaymentC6, _, PaymentC4>(3, 1),
                        tests::dto_price::<PaymentC3, _, PaymentC5>(1, 1),
                    ],
                )
                .unwrap();

            let expected: Vec<BasePrice<PriceCurrencies, BaseCurrency, BaseCurrencies>> = vec![
                tests::base_price::<PaymentC4>(2, 1),
                tests::base_price::<PaymentC5>(2, 1),
                tests::base_price::<PaymentC3>(2, 1),
                tests::base_price::<PaymentC6>(6, 1),
            ];

            let prices: Vec<_> = oracle
                .all_prices_iter(&storage, tree.swap_pairs_df(), env.block.time, 1)
                .collect::<Result<_, _>>()
                .unwrap();

            assert_eq!(prices, expected);
        }
    }
}
