use std::fmt::Debug;

use crate::price::Price;

pub mod count;
pub use count::Count;

#[cfg_attr(any(test, feature = "testing"), derive(PartialEq))]
#[derive(Debug)]
struct AveragePriceBuilder<C, QuoteC>
where
    C: 'static + Debug,
    QuoteC: 'static + Debug,
{
    // An instance of AveragePrice will have at least one Price
    total: Price<C, QuoteC>,
    count: Count,
}

impl<C, QuoteC> AveragePriceBuilder<C, QuoteC>
where
    C: 'static + Debug,
    QuoteC: 'static + Debug,
{
    fn new(initial: Price<C, QuoteC>) -> Self {
        Self {
            total: initial,
            count: Count::ONE,
        }
    }
}

#[cfg(test)]
mod test {
    use currency::test::{SuperGroupTestC1, SuperGroupTestC2};

    use crate::{
        coin::Amount,
        price::{self, Price},
        test::coin,
    };

    use super::{AveragePriceBuilder, Count};

    #[test]
    fn new() {
        assert_eq!(
            AveragePriceBuilder {
                total: price(1, 2),
                count: Count::ONE
            },
            AveragePriceBuilder::new(price(1, 2))
        );
    }

    fn price(amount: Amount, quote_amount: Amount) -> Price<SuperGroupTestC2, SuperGroupTestC1> {
        price::total_of(coin::coin2(amount)).is(coin::coin1(quote_amount))
    }
}
