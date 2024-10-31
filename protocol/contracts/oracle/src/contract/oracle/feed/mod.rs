use std::marker::PhantomData;

use currency::{CurrencyDTO, CurrencyDef, Group, MemberOf};
use finance::price::{base::BasePrice, dto::PriceDTO};
use marketprice::{
    config::Config, market_price::PriceFeeds, ObservationsReadRepo, ObservationsRepo,
};
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

pub struct Feeds<'config, PriceG, BaseC, BaseG, Observations> {
    feeds: PriceFeeds<'config, PriceG, Observations>,
    _base_c: PhantomData<BaseC>,
    _base_g: PhantomData<BaseG>,
}

impl<'config, PriceG, BaseC, BaseG, Observations>
    Feeds<'config, PriceG, BaseC, BaseG, Observations>
{
    pub(crate) fn wipe_out_v2(store: &mut dyn Storage) {
        PriceFeeds::<PriceG, Observations>::wipe_out_v2(store);
    }

    pub(crate) fn with(config: &'config Config, observations: Observations) -> Self {
        Self {
            feeds: PriceFeeds::new(observations, config),
            _base_c: PhantomData,
            _base_g: PhantomData,
        }
    }
}

impl<'config, PriceG, BaseC, BaseG, Observations> Feeds<'config, PriceG, BaseC, BaseG, Observations>
where
    PriceG: Group<TopG = PriceG>,
    BaseC: CurrencyDef,
    BaseC::Group: MemberOf<BaseG> + MemberOf<PriceG>,
    BaseG: Group + MemberOf<PriceG>,
    Observations: ObservationsReadRepo<Group = PriceG>,
{
    pub fn all_prices_iter<'r, 'self_, 'storage, I>(
        &'self_ self,
        swap_pairs_df: I,
        at: Timestamp,
        total_feeders: usize,
    ) -> impl Iterator<Item = PriceResult<PriceG, BaseC, BaseG>> + 'r
    where
        'self_: 'r,
        I: Iterator<Item = SwapLeg<PriceG>> + 'r,
    {
        let cmd: LegCmd<PriceG, BaseC, BaseG, FedPrices<'_, '_, PriceG, Observations>> =
            LegCmd::new(FedPrices::new(&self.feeds, at, total_feeders));

        swap_pairs_df
            .scan(cmd, |cmd, leg: SwapLeg<PriceG>| {
                Some(currency::visit_any_on_currencies(leg.from, leg.to.target, cmd).transpose())
            })
            .flatten()
    }

    pub fn calc_base_price(
        &self,
        tree: &SupportedPairs<PriceG, BaseC>,
        currency: &CurrencyDTO<PriceG>,
        at: Timestamp,
        total_feeders: usize,
    ) -> Result<BasePrice<PriceG, BaseC, BaseG>, ContractError> {
        self.feeds
            .price::<BaseC, _, _>(
                currency::dto::<BaseC, _>(),
                at,
                total_feeders,
                tree.load_path(currency)?,
            )
            .map_err(Into::<ContractError>::into)
    }
}

impl<'config, PriceG, BaseC, BaseG, Observations> Feeds<'config, PriceG, BaseC, BaseG, Observations>
where
    PriceG: Group<TopG = PriceG>,
    BaseC: CurrencyDef,
    BaseC::Group: MemberOf<BaseG> + MemberOf<PriceG>,
    BaseG: Group + MemberOf<PriceG>,
    Observations: ObservationsRepo<Group = PriceG>,
{
    pub(crate) fn feed_prices(
        &mut self,
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
                .feed(block_time, sender_raw, prices)
                .map_err(Into::into)
        }
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
        use marketprice::{config::Config, Repo};
        use sdk::cosmwasm_std::{
            testing::{self, MockStorage},
            Addr, Storage,
        };

        use super::BaseCurrency;
        use crate::{
            contract::oracle::feed::Feeds, state::supported_pairs::SupportedPairs, test_tree, tests,
        };

        const ROOT_NS: &str = "root";

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

            let storage_ptr: &mut dyn Storage = &mut storage;
            let mut oracle = Feeds::with(&config, Repo::new(ROOT_NS, storage_ptr));

            oracle
                .feed_prices(
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
                .all_prices_iter(tree.swap_pairs_df(), env.block.time, 1)
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

            let storage_ptr: &mut dyn Storage = &mut storage;
            let mut oracle = Feeds::with(&config, Repo::new(ROOT_NS, storage_ptr));

            oracle
                .feed_prices(
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
                .all_prices_iter(tree.swap_pairs_df(), env.block.time, 1)
                .collect::<Result<_, _>>()
                .unwrap();

            assert_eq!(prices, expected);
        }
    }
}
