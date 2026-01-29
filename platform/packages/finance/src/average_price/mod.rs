use std::fmt::Debug;

use crate::price::Price;

pub mod count;
pub use count::Count as FeederCount;

#[cfg_attr(any(test, feature = "testing"), derive(PartialEq))]
#[derive(Debug)]
struct AveragePrice<C, QuoteC>
where
    C: 'static + Debug,
    QuoteC: 'static + Debug,
{
    // An instance of AveragePrice will have at least one Price
    total: Price<C, QuoteC>,
    // > 0; TODO: replace with Count
    count: u32,
}

impl<C, QuoteC> AveragePrice<C, QuoteC>
where
    C: 'static + Debug,
    QuoteC: 'static + Debug,
{
    fn new(initial: Price<C, QuoteC>) -> Self {
        Self {
            total: initial,
            count: 1,
        }
    }
}

#[cfg(test)]
mod test {
    use currency::test::SuperGroupTestC2;

    use crate::{
        super::AveragePrice,
        average_price::AveragePrice,
        coin::Amount,
        price::{self, Price, PriceBuilder},
        test::coin,
    };

    #[test]
    fn new() {
        assert_eq!(
            AveragePrice {
                total: price(1, 2),
                count: 1
            },
            AveragePrice::new(price(1, 2))
        );
    }

    fn price(amount: Amount, quote_amount: Amount) -> Price<SuperGroupTestC2, SuperGroupTestC1> {
        price::total_of(coin::coin2(amount)).is(coin::coin1(quote_amount))
    }
}
