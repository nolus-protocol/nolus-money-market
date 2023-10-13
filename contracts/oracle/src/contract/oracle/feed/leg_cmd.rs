use serde::de::DeserializeOwned;

use currency::{AnyVisitorPair, Currency};
use finance::price::{base::BasePrice, Price};
use swap::SwapGroup;

use crate::ContractError;

use super::price_querier::PriceQuerier;

pub struct LegCmd<OracleBase, Querier>
where
    OracleBase: Currency,
    Querier: PriceQuerier,
{
    price_querier: Querier,
    stack: Vec<BasePrice<SwapGroup, OracleBase>>,
}

impl<OracleBase, Querier> LegCmd<OracleBase, Querier>
where
    OracleBase: Currency,
    Querier: PriceQuerier,
{
    pub fn new(price_querier: Querier, stack: Vec<BasePrice<SwapGroup, OracleBase>>) -> Self {
        Self {
            price_querier,
            stack,
        }
    }
}

impl<OracleBase, Querier> AnyVisitorPair for &mut LegCmd<OracleBase, Querier>
where
    OracleBase: Currency + DeserializeOwned,
    Querier: PriceQuerier,
{
    type Output = Option<BasePrice<SwapGroup, OracleBase>>;
    type Error = ContractError;

    fn on<B, Q>(self) -> Result<Self::Output, Self::Error>
    where
        B: Currency + DeserializeOwned,
        Q: Currency + DeserializeOwned,
    {
        // tries to find price for non empty stack (in a branch of the tree)
        // covers both normal flow and NoPrice cases
        let branch_price = self
            .stack
            .iter()
            .enumerate()
            .rev()
            .find_map(|(i, parent_bprice)| {
                Price::<Q, OracleBase>::try_from(parent_bprice)
                    .ok()
                    .map(|parent_price| {
                        self.price_querier
                            .price::<B, Q>()
                            .map(|res_price| res_price.map(|price| (i + 1, price * parent_price)))
                    })
            })
            .transpose()
            .map(Option::flatten)?;

        // Fallback for the root case: Q==OracleBase.
        // Here we rely on the SupprtedPairs tree invariant (unique currencies),
        // if we can find price::<B, OracleBase>, the algorithm is at the root point of the tree,
        // reseting the stack.
        let idx_price = branch_price.map_or_else(
            || {
                self.price_querier
                    .price::<B, OracleBase>()
                    .map(|res_price| res_price.map(|price| (0, price)))
            },
            |idx_price| Ok(Some(idx_price)),
        )?;

        if let Some((idx, price)) = idx_price {
            self.stack.truncate(idx);
            self.stack.push(price.into());
        }

        Ok(idx_price.map(|(_, price)| price.into()))
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use currency::test::{PaymentC1, PaymentC3, PaymentC4, PaymentC5, PaymentC6};

    use crate::tests::{self, TheCurrency};

    use super::{super::test::TestFeeds, *};

    #[test]
    fn leg_cmd_normal_flow() {
        let mut feeds = TestFeeds(HashMap::new());
        feeds.add::<PaymentC4, TheCurrency>(1, 1);
        feeds.add::<PaymentC1, TheCurrency>(2, 1);
        feeds.add::<PaymentC5, PaymentC4>(2, 1);

        let mut cmd = LegCmd::<TheCurrency, _> {
            price_querier: feeds.clone(),
            stack: vec![],
        };
        assert_eq!(
            cmd.on::<PaymentC4, TheCurrency>(),
            Ok(Some(tests::base_price::<PaymentC4>(1, 1)))
        );
        assert_eq!(cmd.stack, vec![tests::base_price::<PaymentC4>(1, 1)]);

        // child
        assert_eq!(
            cmd.on::<PaymentC5, PaymentC4>(),
            Ok(Some(tests::base_price::<PaymentC5>(2, 1)))
        );
        assert_eq!(
            cmd.stack,
            vec![
                tests::base_price::<PaymentC4>(1, 1),
                tests::base_price::<PaymentC5>(2, 1)
            ]
        );

        // hop to the next branch
        assert_eq!(
            cmd.on::<PaymentC1, TheCurrency>(),
            Ok(Some(tests::base_price::<PaymentC1>(2, 1)))
        );
        assert_eq!(cmd.stack, vec![tests::base_price::<PaymentC1>(2, 1)]);
    }

    #[test]
    fn no_price_in_stack() {
        let mut feeds = TestFeeds(HashMap::new());
        feeds.add::<PaymentC4, TheCurrency>(2, 1);

        let mut cmd = LegCmd::<TheCurrency, _> {
            price_querier: feeds.clone(),
            stack: vec![tests::base_price::<PaymentC4>(2, 1)],
        };

        assert_eq!(cmd.on::<PaymentC6, PaymentC4>(), Ok(None));
        assert_eq!(cmd.stack, vec![tests::base_price::<PaymentC4>(2, 1),]);
    }

    #[test]
    fn no_parent_price_in_stack() {
        let mut feeds = TestFeeds(HashMap::new());
        feeds.add::<PaymentC4, TheCurrency>(2, 1);
        feeds.add::<PaymentC3, PaymentC5>(3, 1);

        let mut cmd = LegCmd::<TheCurrency, _> {
            price_querier: feeds.clone(),
            stack: vec![tests::base_price::<PaymentC4>(2, 1)],
        };

        assert_eq!(cmd.on::<PaymentC3, PaymentC5>(), Ok(None));
        assert_eq!(cmd.stack, vec![tests::base_price::<PaymentC4>(2, 1),]);
    }

    #[test]
    fn hop_parent_in_stack() {
        let mut feeds = TestFeeds(HashMap::new());
        feeds.add::<PaymentC4, TheCurrency>(2, 1);
        feeds.add::<PaymentC3, PaymentC4>(3, 1);
        feeds.add::<PaymentC6, PaymentC4>(4, 1);

        let mut cmd = LegCmd::<TheCurrency, _> {
            price_querier: feeds.clone(),
            stack: vec![
                tests::base_price::<PaymentC4>(2, 1),
                tests::base_price::<PaymentC3>(6, 1),
            ],
        };

        assert_eq!(
            cmd.on::<PaymentC6, PaymentC4>(),
            Ok(Some(tests::base_price::<PaymentC6>(8, 1)))
        );
        assert_eq!(
            cmd.stack,
            vec![
                tests::base_price::<PaymentC4>(2, 1),
                tests::base_price::<PaymentC6>(8, 1),
            ]
        );
    }

    #[test]
    fn hop_to_root() {
        let mut feeds = TestFeeds(HashMap::new());
        feeds.add::<PaymentC4, TheCurrency>(2, 1);
        feeds.add::<PaymentC6, TheCurrency>(4, 1);

        let mut cmd = LegCmd::<TheCurrency, _> {
            price_querier: feeds.clone(),
            stack: vec![tests::base_price::<PaymentC4>(2, 1)],
        };

        assert_eq!(
            cmd.on::<PaymentC6, TheCurrency>(),
            Ok(Some(tests::base_price::<PaymentC6>(4, 1)))
        );
        assert_eq!(cmd.stack, vec![tests::base_price::<PaymentC6>(4, 1),]);
    }

    #[test]
    fn price_root_with_empty_stack() {
        let mut feeds = TestFeeds(HashMap::new());
        feeds.add::<PaymentC6, TheCurrency>(4, 1);

        let mut cmd = LegCmd::<TheCurrency, _> {
            price_querier: feeds.clone(),
            stack: vec![],
        };

        assert_eq!(
            cmd.on::<PaymentC6, TheCurrency>(),
            Ok(Some(tests::base_price::<PaymentC6>(4, 1)))
        );
        assert_eq!(cmd.stack, vec![tests::base_price::<PaymentC6>(4, 1),]);
    }

    #[test]
    fn no_price_at_root() {
        let mut feeds = TestFeeds(HashMap::new());
        feeds.add::<PaymentC4, TheCurrency>(2, 1);

        let mut cmd = LegCmd::<TheCurrency, _> {
            price_querier: feeds.clone(),
            stack: vec![tests::base_price::<PaymentC4>(2, 1)],
        };

        assert_eq!(cmd.on::<PaymentC6, TheCurrency>(), Ok(None));
        // cleaned on the next successful iteration
        assert_eq!(cmd.stack, vec![tests::base_price::<PaymentC4>(2, 1),]);
    }
}
