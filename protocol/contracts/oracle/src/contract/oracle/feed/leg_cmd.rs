use currency::{AnyVisitorPair, Currency, CurrencyDTO, CurrencyDef, Group, MemberOf};
use finance::price::{Price, base::BasePrice};

use crate::error::Error;

use super::price_querier::PriceQuerier;

pub struct LegCmd<PriceG, BaseC, BaseG, Querier>
where
    PriceG: Group,
    BaseC: CurrencyDef,
    BaseC::Group: MemberOf<BaseG> + MemberOf<PriceG::TopG>,
    BaseG: Group,
    Querier: PriceQuerier,
{
    price_querier: Querier,
    stack: Vec<BasePrice<PriceG, BaseC, BaseG>>,
}

impl<PriceG, BaseC, BaseG, Querier> LegCmd<PriceG, BaseC, BaseG, Querier>
where
    PriceG: Group<TopG = PriceG>,
    BaseC: CurrencyDef,
    BaseC::Group: MemberOf<BaseG> + MemberOf<PriceG>,
    BaseG: Group + MemberOf<PriceG>,
    Querier: PriceQuerier<CurrencyGroup = PriceG>,
{
    pub fn new(price_querier: Querier) -> Self {
        Self {
            price_querier,
            stack: vec![Price::<BaseC, BaseC>::identity().into()],
        }
    }

    // Price<TargetC, BaseC> = Price<TargetC, QuoteC> * Price<QuoteC, BaseC>
    fn extend_quote_to_target<T, Q>(
        &self,
        target_c: &CurrencyDTO<PriceG>,
        quote_c: &CurrencyDTO<PriceG>,
        quote_price: Price<Q, BaseC>,
    ) -> Result<Option<BasePrice<PriceG, BaseC, BaseG>>, Error<PriceG>>
    where
        T: Currency + MemberOf<PriceG>,
        Q: Currency + MemberOf<PriceG>,
    {
        self.price_querier
            .price::<T, Q>(target_c, quote_c)
            .and_then(|target_quote_price| {
                target_quote_price
                    .map(|price| {
                        (price * quote_price)
                            .ok_or_else(Error::PriceMultiplicationOverflow)
                            .map(|not_overflown| BasePrice::from_price(&not_overflown, *target_c))
                    })
                    .transpose()
            })
    }
}

impl<PriceG, BaseC, BaseG, Querier> AnyVisitorPair for &mut LegCmd<PriceG, BaseC, BaseG, Querier>
where
    PriceG: Group<TopG = PriceG>,
    BaseC: CurrencyDef,
    BaseC::Group: MemberOf<BaseG> + MemberOf<PriceG>,
    BaseG: Group + MemberOf<PriceG>,
    Querier: PriceQuerier<CurrencyGroup = PriceG>,
{
    type VisitedG = PriceG;

    type Outcome = Result<Option<BasePrice<PriceG, BaseC, BaseG>>, Error<PriceG>>;

    fn on<T, Q>(
        self,
        target_c: &CurrencyDTO<Self::VisitedG>,
        quote_c: &CurrencyDTO<Self::VisitedG>,
    ) -> Self::Outcome
    where
        T: Currency + MemberOf<Self::VisitedG>,
        Q: Currency + MemberOf<Self::VisitedG>,
    {
        // tries to find price for non empty stack (in a branch of the tree)
        // covers both lack-of-a-price and normal flow usecases, including multiplication overflows.
        let idx_target_price = self
            .stack
            .iter()
            .enumerate()
            .rev()
            .find_map(|(i, quote_bprice)| {
                quote_bprice
                    .try_as_specific::<Q, Self::VisitedG>(quote_c)
                    .ok()
                    .map(|quote_price| (i, quote_price))
            })
            .map(|(idx_quote_price, quote_price)| {
                self.extend_quote_to_target::<T, Q>(target_c, quote_c, quote_price)
                    .map(|may_target_price| {
                        may_target_price.map(|target_price| (idx_quote_price + 1, target_price))
                    })
                    .transpose()
            })
            .flatten()
            .transpose();

        idx_target_price.map(|a| {
            a.map(|(idx, target_price)| {
                self.stack.truncate(idx);
                self.stack.push(target_price);
                target_price
            })
        })
    }
}

#[cfg(all(feature = "internal.test.contract", test))]
mod test {
    use std::collections::HashMap;

    use currencies::{
        Lpn as BaseCurrency, Lpns as BaseCurrencies, PaymentGroup as PaymentCurrencies,
        testing::{PaymentC1, PaymentC3, PaymentC4, PaymentC5, PaymentC6},
    };

    use crate::tests;

    use super::{super::test::TestFeeds, *};

    #[test]
    fn leg_cmd_normal_flow() {
        let mut feeds = TestFeeds(HashMap::new());
        feeds.add::<PaymentC4, BaseCurrency>(1, 1);
        feeds.add::<PaymentC1, BaseCurrency>(2, 1);
        feeds.add::<PaymentC5, PaymentC4>(2, 1);

        let mut cmd =
            LegCmd::<PaymentCurrencies, BaseCurrency, BaseCurrencies, _>::new(feeds.clone());
        assert_eq!(
            cmd.on::<PaymentC4, BaseCurrency>(
                &currency::dto::<PaymentC4, PaymentCurrencies>(),
                &currency::dto::<BaseCurrency, _>()
            ),
            Ok(Some(tests::base_price::<PaymentC4>(1, 1)))
        );

        // child
        assert_eq!(
            cmd.on::<PaymentC5, PaymentC4>(
                &currency::dto::<PaymentC5, _>(),
                &currency::dto::<PaymentC4, _>()
            ),
            Ok(Some(tests::base_price::<PaymentC5>(2, 1)))
        );

        // hop to the next branch
        assert_eq!(
            cmd.on::<PaymentC1, BaseCurrency>(
                &currency::dto::<PaymentC1, _>(),
                &currency::dto::<BaseCurrency, _>()
            ),
            Ok(Some(tests::base_price::<PaymentC1>(2, 1)))
        );
    }

    #[test]
    fn no_price_in_stack() {
        let mut feeds = TestFeeds(HashMap::new());
        feeds.add::<PaymentC4, BaseCurrency>(2, 1);

        let mut cmd =
            LegCmd::<PaymentCurrencies, BaseCurrency, BaseCurrencies, _>::new(feeds.clone());

        assert_eq!(
            cmd.on::<PaymentC6, PaymentC4>(
                &currency::dto::<PaymentC6, _>(),
                &currency::dto::<PaymentC4, _>()
            ),
            Ok(None)
        );

        assert_eq!(
            cmd.on::<PaymentC4, BaseCurrency>(
                &currency::dto::<PaymentC4, _>(),
                &currency::dto::<BaseCurrency, _>()
            ),
            Ok(Some(tests::base_price::<PaymentC4>(2, 1)))
        );
    }

    #[test]
    fn no_parent_price_in_stack() {
        let mut feeds = TestFeeds(HashMap::new());
        feeds.add::<PaymentC4, BaseCurrency>(2, 1);
        feeds.add::<PaymentC5, PaymentC4>(5, 1);
        feeds.add::<PaymentC3, PaymentC5>(3, 1);

        let mut cmd =
            LegCmd::<PaymentCurrencies, BaseCurrency, BaseCurrencies, _>::new(feeds.clone());

        assert_eq!(
            cmd.on::<PaymentC3, PaymentC5>(
                &currency::dto::<PaymentC3, _>(),
                &currency::dto::<PaymentC5, _>()
            ),
            Ok(None)
        );
        assert_eq!(
            cmd.on::<PaymentC5, PaymentC4>(
                &currency::dto::<PaymentC5, _>(),
                &currency::dto::<PaymentC4, _>(),
            ),
            Ok(None)
        );

        assert_eq!(
            cmd.on::<PaymentC4, BaseCurrency>(
                &currency::dto::<PaymentC4, _>(),
                &currency::dto::<BaseCurrency, _>(),
            ),
            Ok(Some(tests::base_price::<PaymentC4>(2, 1)))
        );
        assert_eq!(
            cmd.on::<PaymentC5, PaymentC4>(
                &currency::dto::<PaymentC5, _>(),
                &currency::dto::<PaymentC4, _>(),
            ),
            Ok(Some(tests::base_price::<PaymentC5>(10, 1)))
        );
        assert_eq!(
            cmd.on::<PaymentC3, PaymentC5>(
                &currency::dto::<PaymentC3, _>(),
                &currency::dto::<PaymentC5, _>(),
            ),
            Ok(Some(tests::base_price::<PaymentC3>(30, 1)))
        );
    }

    #[test]
    fn hop_parent_in_stack() {
        let mut feeds = TestFeeds(HashMap::new());
        feeds.add::<PaymentC4, BaseCurrency>(2, 1);
        feeds.add::<PaymentC5, PaymentC4>(3, 1);
        feeds.add::<PaymentC6, PaymentC4>(4, 1);

        let mut cmd =
            LegCmd::<PaymentCurrencies, BaseCurrency, BaseCurrencies, _>::new(feeds.clone());

        assert_eq!(
            cmd.on::<PaymentC4, BaseCurrency>(
                &currency::dto::<PaymentC4, _>(),
                &currency::dto::<BaseCurrency, _>(),
            ),
            Ok(Some(tests::base_price::<PaymentC4>(2, 1)))
        );

        assert_eq!(
            cmd.on::<PaymentC6, PaymentC4>(
                &currency::dto::<PaymentC6, _>(),
                &currency::dto::<PaymentC4, _>(),
            ),
            Ok(Some(tests::base_price::<PaymentC6>(8, 1)))
        );

        assert_eq!(
            cmd.on::<PaymentC5, PaymentC4>(
                &currency::dto::<PaymentC5, _>(),
                &currency::dto::<PaymentC4, _>(),
            ),
            Ok(Some(tests::base_price::<PaymentC5>(6, 1)))
        );

        assert_eq!(
            cmd.on::<PaymentC6, PaymentC4>(
                &currency::dto::<PaymentC6, _>(),
                &currency::dto::<PaymentC4, _>(),
            ),
            Ok(Some(tests::base_price::<PaymentC6>(8, 1)))
        );
    }

    #[test]
    fn hop_to_root() {
        let mut feeds = TestFeeds(HashMap::new());
        feeds.add::<PaymentC4, BaseCurrency>(2, 1);
        feeds.add::<PaymentC1, BaseCurrency>(4, 1);

        let mut cmd =
            LegCmd::<PaymentCurrencies, BaseCurrency, BaseCurrencies, _>::new(feeds.clone());

        assert_eq!(
            cmd.on::<PaymentC1, BaseCurrency>(
                &currency::dto::<PaymentC1, _>(),
                &currency::dto::<BaseCurrency, _>(),
            ),
            Ok(Some(tests::base_price::<PaymentC1>(4, 1)))
        );
    }

    #[test]
    fn price_root_with_empty_stack() {
        let mut feeds = TestFeeds(HashMap::new());
        feeds.add::<PaymentC1, BaseCurrency>(4, 1);

        let mut cmd =
            LegCmd::<PaymentCurrencies, BaseCurrency, BaseCurrencies, _>::new(feeds.clone());

        assert_eq!(
            Ok(Some(tests::base_price::<PaymentC1>(4, 1))),
            cmd.on::<PaymentC1, BaseCurrency>(
                &currency::dto::<PaymentC1, _>(),
                &currency::dto::<BaseCurrency, _>(),
            ),
        );
    }

    #[test]
    fn no_price_at_root() {
        let mut feeds = TestFeeds(HashMap::new());
        feeds.add::<PaymentC4, BaseCurrency>(2, 1);

        let mut cmd =
            LegCmd::<PaymentCurrencies, BaseCurrency, BaseCurrencies, _>::new(feeds.clone());

        assert_eq!(
            cmd.on::<PaymentC1, BaseCurrency>(
                &currency::dto::<PaymentC1, _>(),
                &currency::dto::<BaseCurrency, _>(),
            ),
            Ok(None)
        );
    }
}
