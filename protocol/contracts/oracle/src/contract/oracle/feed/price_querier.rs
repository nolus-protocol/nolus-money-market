use currency::{Currency, CurrencyDTO, Group, MemberOf};
use finance::price::Price;
use marketprice::{error::PriceFeedsError, market_price::PriceFeeds, ObservationsReadRepo};
use sdk::cosmwasm_std::Timestamp;

use crate::ContractError;

pub struct FedPrices<'a, 'config, G, Observations>
where
    G: Group,
{
    feeds: &'a PriceFeeds<'config, G, Observations>,
    at: Timestamp,
    total_feeders: usize,
}

impl<'a, 'config, G, Observations> FedPrices<'a, 'config, G, Observations>
where
    G: Group,
{
    pub fn new(
        feeds: &'a PriceFeeds<'config, G, Observations>,
        at: Timestamp,
        total_feeders: usize,
    ) -> Self {
        Self {
            feeds,
            at,
            total_feeders,
        }
    }
}

pub trait PriceQuerier {
    type CurrencyGroup: Group;

    fn price<C, QuoteC>(
        &self,
        amount_c: &CurrencyDTO<Self::CurrencyGroup>,
        quote_c: &CurrencyDTO<Self::CurrencyGroup>,
    ) -> Result<Option<Price<C, QuoteC>>, ContractError>
    where
        C: Currency + MemberOf<Self::CurrencyGroup>,
        QuoteC: Currency + MemberOf<Self::CurrencyGroup>;
}

impl<G, Observations> PriceQuerier for FedPrices<'_, '_, G, Observations>
where
    G: Group<TopG = G>,
    Observations: ObservationsReadRepo<Group = G>,
{
    type CurrencyGroup = G;

    fn price<C, QuoteC>(
        &self,
        amount_c: &CurrencyDTO<Self::CurrencyGroup>,
        quote_c: &CurrencyDTO<Self::CurrencyGroup>,
    ) -> Result<Option<Price<C, QuoteC>>, ContractError>
    where
        C: Currency + MemberOf<Self::CurrencyGroup>,
        QuoteC: Currency + MemberOf<Self::CurrencyGroup>,
    {
        let price = self
            .feeds
            .price_of_feed(amount_c, quote_c, self.at, self.total_feeders);
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
    price
        .map(Some)
        .or_else(|err| match err {
            PriceFeedsError::NoPrice() => Ok(None),
            _ => Err(err),
        })
        .map_err(Into::into)
}

#[cfg(test)]
mod test {
    use currencies::testing::{PaymentC3, PaymentC7};
    use finance::{coin::Coin, price::total_of};

    use super::*;

    #[test]
    fn test_maybe_price() {
        let price = total_of(Coin::<PaymentC3>::new(1)).is(Coin::<PaymentC7>::new(2));
        assert_eq!(maybe_price(Ok(price)), Ok(Some(price)));
        assert_eq!(
            maybe_price::<PaymentC3, PaymentC7>(Err(PriceFeedsError::NoPrice())),
            Ok(None)
        );
        // other errors
        let err_msg: String = "test_err".into();
        assert_eq!(
            maybe_price::<PaymentC3, PaymentC7>(Err(PriceFeedsError::Configuration(
                err_msg.clone()
            ))),
            Err(PriceFeedsError::Configuration(err_msg).into())
        );
    }
}
