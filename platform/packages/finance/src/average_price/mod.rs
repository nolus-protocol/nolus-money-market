use std::fmt::Debug;

#[cfg(test)]
use crate::average_price::count::Unit;
use crate::{price::Price, ratio::SimpleFraction};

pub mod count;
pub use count::Count;

#[cfg_attr(test, derive(PartialEq))]
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

    #[cfg(test)]
    fn test_new(initial: Price<C, QuoteC>, count: Unit) -> Self {
        let count = Count::test_new(count);
        count.assert_nonzero();
        Self {
            total: initial,
            count,
        }
    }

    pub fn calculate(self) -> Option<Price<C, QuoteC>> {
        self.total.lossy_mul(Self::reciprocal_count(self.count))
    }

    fn reciprocal_count(count: Count) -> SimpleFraction<Count> {
        count
            .try_into_reciprocal()
            .map(SimpleFraction::from)
            .expect("Count in AveragePriceBuilder is never zero")
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

    #[test]
    fn price_calculate() {
        let builder = AveragePriceBuilder::test_new(price(3, 4), 5);
        let exp = price(15, 4);
        assert_eq!(Some(exp), builder.calculate());
    }

    fn price(amount: Amount, quote_amount: Amount) -> Price<SuperGroupTestC2, SuperGroupTestC1> {
        price::total_of(coin::coin2(amount)).is(coin::coin1(quote_amount))
    }
}
