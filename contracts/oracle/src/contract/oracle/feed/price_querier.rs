use serde::de::DeserializeOwned;

use finance::{currency::Currency, price::Price};
use marketprice::{error::PriceFeedsError, market_price::PriceFeeds};
use sdk::cosmwasm_std::{Storage, Timestamp};

use crate::ContractError;

pub struct FedPrices<'a> {
    storage: &'a dyn Storage,
    feeds: &'a PriceFeeds<'static>,
    at: Timestamp,
    total_feeders: usize,
}

impl<'a> FedPrices<'a> {
    pub fn new(
        storage: &'a dyn Storage,
        feeds: &'a PriceFeeds<'static>,
        at: Timestamp,
        total_feeders: usize,
    ) -> Self {
        Self {
            feeds,
            at,
            total_feeders,
            storage,
        }
    }
}

pub trait PriceQuerier {
    fn price<B, Q>(&self) -> Result<Option<Price<B, Q>>, ContractError>
    where
        B: Currency + DeserializeOwned,
        Q: Currency + DeserializeOwned;
}

impl<'a> PriceQuerier for FedPrices<'a> {
    fn price<B, Q>(&self) -> Result<Option<Price<B, Q>>, ContractError>
    where
        B: Currency + DeserializeOwned,
        Q: Currency + DeserializeOwned,
    {
        let price = self
            .feeds
            .price_of_feed(self.storage, self.at, self.total_feeders);
        maybe_price(price)
    }
}

fn maybe_price<B, Q>(
    price: Result<Price<B, Q>, PriceFeedsError>,
) -> Result<Option<Price<B, Q>>, ContractError>
where
    B: Currency,
    Q: Currency,
{
    Ok(price.map(Some).or_else(|err| match err {
        PriceFeedsError::NoPrice() => Ok(None),
        _ => Err(err),
    })?)
}

#[cfg(test)]
mod test {
    use ::currency::lease::{Atom, Weth};
    use finance::{coin::Coin, price::total_of};

    use super::*;

    #[test]
    fn test_maybe_price() {
        let price = total_of(Coin::<Atom>::new(1)).is(Coin::<Weth>::new(2));
        assert_eq!(maybe_price(Ok(price)), Ok(Some(price)));
        assert_eq!(
            maybe_price::<Atom, Weth>(Err(PriceFeedsError::NoPrice())),
            Ok(None)
        );
        // other errors
        assert_eq!(
            maybe_price::<Atom, Weth>(Err(PriceFeedsError::InvalidPrice())),
            Err(PriceFeedsError::InvalidPrice().into())
        );
    }
}
