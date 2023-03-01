use super::price_querier::PriceQuerier;
use crate::ContractError;
use finance::{
    currency::{AnyVisitorPair, Currency},
    price::{base::BasePrice, Price},
};
use serde::de::DeserializeOwned;
use swap::SwapGroup;

pub struct LegCmd<OracleBase, Querier>
where
    OracleBase: Currency,
    Querier: PriceQuerier,
{
    pub price_querier: Querier,
    pub stack: Vec<BasePrice<SwapGroup, OracleBase>>,
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
    use super::*;

    use std::collections::HashMap;

    use crate::{
        contract::feed::test::TestFeeds,
        tests::{self, TheCurrency},
    };
    use ::currency::lease::{Atom, Cro, Juno, Osmo, Wbtc, Weth};

    #[test]
    fn leg_cmd_normal() {
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
    fn leg_cmd_missing_price() {
        let mut feeds = TestFeeds(HashMap::new());
        feeds.add::<Wbtc, TheCurrency>(1, 1);
        feeds.add::<Atom, TheCurrency>(2, 1);
        feeds.add::<Weth, Wbtc>(2, 1);
        feeds.add::<Osmo, Weth>(1, 1);
        feeds.add::<Cro, Osmo>(3, 1);

        feeds.add::<Juno, Wbtc>(1, 1);

        let mut cmd = LegCmd::<TheCurrency, _> {
            price_querier: feeds.clone(),
            stack: vec![
                tests::base_price::<Wbtc>(1, 1),
                tests::base_price::<Weth>(2, 1),
            ],
        };

        // no price
        assert_eq!(cmd.on::<Cro, Weth>(), Ok(None));
        assert_eq!(
            cmd.stack,
            vec![
                tests::base_price::<Wbtc>(1, 1),
                tests::base_price::<Weth>(2, 1)
            ]
        );

        // recover, hop to the top child, clean the stack
        assert_eq!(
            cmd.on::<Atom, TheCurrency>(),
            Ok(Some(tests::base_price::<Atom>(2, 1)))
        );
        assert_eq!(cmd.stack, vec![tests::base_price::<Atom>(2, 1)]);

        let mut cmd = LegCmd::<TheCurrency, _> {
            price_querier: feeds.clone(),
            stack: vec![
                tests::base_price::<Wbtc>(1, 1),
                tests::base_price::<Weth>(2, 1),
            ],
        };

        // recover, hop to the close child, clean the stack
        assert_eq!(
            cmd.on::<Juno, Wbtc>(),
            Ok(Some(tests::base_price::<Juno>(1, 1)))
        );
        assert_eq!(
            cmd.stack,
            vec![
                tests::base_price::<Wbtc>(1, 1),
                tests::base_price::<Juno>(1, 1)
            ]
        );
    }
}
