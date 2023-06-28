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

    use ::currency::lease::{Atom, Cro, Juno, Wbtc, Weth};

    use crate::tests::{self, TheCurrency};

    use super::{super::test::TestFeeds, *};

    #[test]
    fn leg_cmd_normal_flow() {
        let mut feeds = TestFeeds(HashMap::new());
        feeds.add::<Wbtc, TheCurrency>(1, 1);
        feeds.add::<Atom, TheCurrency>(2, 1);
        feeds.add::<Weth, Wbtc>(2, 1);

        let mut cmd = LegCmd::<TheCurrency, _> {
            price_querier: feeds.clone(),
            stack: vec![],
        };
        assert_eq!(
            cmd.on::<Wbtc, TheCurrency>(),
            Ok(Some(tests::base_price::<Wbtc>(1, 1)))
        );
        assert_eq!(cmd.stack, vec![tests::base_price::<Wbtc>(1, 1)]);

        // child
        assert_eq!(
            cmd.on::<Weth, Wbtc>(),
            Ok(Some(tests::base_price::<Weth>(2, 1)))
        );
        assert_eq!(
            cmd.stack,
            vec![
                tests::base_price::<Wbtc>(1, 1),
                tests::base_price::<Weth>(2, 1)
            ]
        );

        // hop to the next branch
        assert_eq!(
            cmd.on::<Atom, TheCurrency>(),
            Ok(Some(tests::base_price::<Atom>(2, 1)))
        );
        assert_eq!(cmd.stack, vec![tests::base_price::<Atom>(2, 1)]);
    }

    #[test]
    fn no_price_in_stack() {
        let mut feeds = TestFeeds(HashMap::new());
        feeds.add::<Wbtc, TheCurrency>(2, 1);

        let mut cmd = LegCmd::<TheCurrency, _> {
            price_querier: feeds.clone(),
            stack: vec![tests::base_price::<Wbtc>(2, 1)],
        };

        assert_eq!(cmd.on::<Cro, Wbtc>(), Ok(None));
        assert_eq!(cmd.stack, vec![tests::base_price::<Wbtc>(2, 1),]);
    }

    #[test]
    fn no_parent_price_in_stack() {
        let mut feeds = TestFeeds(HashMap::new());
        feeds.add::<Wbtc, TheCurrency>(2, 1);
        feeds.add::<Juno, Weth>(3, 1);

        let mut cmd = LegCmd::<TheCurrency, _> {
            price_querier: feeds.clone(),
            stack: vec![tests::base_price::<Wbtc>(2, 1)],
        };

        assert_eq!(cmd.on::<Juno, Weth>(), Ok(None));
        assert_eq!(cmd.stack, vec![tests::base_price::<Wbtc>(2, 1),]);
    }

    #[test]
    fn hop_parent_in_stack() {
        let mut feeds = TestFeeds(HashMap::new());
        feeds.add::<Wbtc, TheCurrency>(2, 1);
        feeds.add::<Juno, Wbtc>(3, 1);
        feeds.add::<Cro, Wbtc>(4, 1);

        let mut cmd = LegCmd::<TheCurrency, _> {
            price_querier: feeds.clone(),
            stack: vec![
                tests::base_price::<Wbtc>(2, 1),
                tests::base_price::<Juno>(6, 1),
            ],
        };

        assert_eq!(
            cmd.on::<Cro, Wbtc>(),
            Ok(Some(tests::base_price::<Cro>(8, 1)))
        );
        assert_eq!(
            cmd.stack,
            vec![
                tests::base_price::<Wbtc>(2, 1),
                tests::base_price::<Cro>(8, 1),
            ]
        );
    }

    #[test]
    fn hop_to_root() {
        let mut feeds = TestFeeds(HashMap::new());
        feeds.add::<Wbtc, TheCurrency>(2, 1);
        feeds.add::<Cro, TheCurrency>(4, 1);

        let mut cmd = LegCmd::<TheCurrency, _> {
            price_querier: feeds.clone(),
            stack: vec![tests::base_price::<Wbtc>(2, 1)],
        };

        assert_eq!(
            cmd.on::<Cro, TheCurrency>(),
            Ok(Some(tests::base_price::<Cro>(4, 1)))
        );
        assert_eq!(cmd.stack, vec![tests::base_price::<Cro>(4, 1),]);
    }

    #[test]
    fn price_root_with_empty_stack() {
        let mut feeds = TestFeeds(HashMap::new());
        feeds.add::<Cro, TheCurrency>(4, 1);

        let mut cmd = LegCmd::<TheCurrency, _> {
            price_querier: feeds.clone(),
            stack: vec![],
        };

        assert_eq!(
            cmd.on::<Cro, TheCurrency>(),
            Ok(Some(tests::base_price::<Cro>(4, 1)))
        );
        assert_eq!(cmd.stack, vec![tests::base_price::<Cro>(4, 1),]);
    }

    #[test]
    fn no_price_at_root() {
        let mut feeds = TestFeeds(HashMap::new());
        feeds.add::<Wbtc, TheCurrency>(2, 1);

        let mut cmd = LegCmd::<TheCurrency, _> {
            price_querier: feeds.clone(),
            stack: vec![tests::base_price::<Wbtc>(2, 1)],
        };

        assert_eq!(cmd.on::<Cro, TheCurrency>(), Ok(None));
        // cleaned on the next successful iteration
        assert_eq!(cmd.stack, vec![tests::base_price::<Wbtc>(2, 1),]);
    }
}
