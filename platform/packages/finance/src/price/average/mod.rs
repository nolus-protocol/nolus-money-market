use std::fmt::Debug;

use crate::{price::Price, ratio::SimpleFraction};
#[cfg(test)]
use count::Unit;

pub mod count;
pub use count::Count;

#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug)]
pub struct PriceAccumulator<C, QuoteC>
where
    C: 'static + Debug,
    QuoteC: 'static + Debug,
{
    // An instance of AveragePrice will have at least one Price
    total: Price<C, QuoteC>,
    count: Count,
}

impl<C, QuoteC> PriceAccumulator<C, QuoteC>
where
    C: 'static + Debug,
    QuoteC: 'static + Debug,
{
    pub fn init_with(initial: Price<C, QuoteC>) -> Self {
        Self::new(initial, Count::ONE)
    }

    pub fn try_add(self, price: Price<C, QuoteC>) -> Option<Self> {
        self.total
            .lossy_add(price)
            .zip(self.count.try_increment())
            .map(|(accumulated_price, incremented)| Self::new(accumulated_price, incremented))
    }

    pub fn average(&self) -> Option<Price<C, QuoteC>> {
        self.total.lossy_mul(Self::reciprocal_count(self.count))
    }

    fn new(initial: Price<C, QuoteC>, count: Count) -> Self {
        debug_assert!(!count.is_zero(), "Count must be non-zero");
        Self {
            total: initial,
            count,
        }
    }

    fn reciprocal_count(count: Count) -> SimpleFraction<Count> {
        count
            .try_into_reciprocal()
            .map(SimpleFraction::from)
            .expect("Count in PriceAccumulator is never zero")
    }

    #[cfg(test)]
    fn test_new(initial: Price<C, QuoteC>, count: Unit) -> Self {
        let count = Count::test_new(count);
        Self::new(initial, count)
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

    use super::{Count, PriceAccumulator, count::Unit};

    #[test]
    fn init_with() {
        assert_eq!(
            PriceAccumulator {
                total: price(1, 2),
                count: Count::ONE
            },
            PriceAccumulator::init_with(price(1, 2))
        );
    }

    #[test]
    fn add() {
        let builder = PriceAccumulator::test_new(price(1, 2), 2);
        // second prices added
        let may_actual = builder.try_add(price(3, 4));
        assert_eq!(
            Some(PriceAccumulator::test_new(price(3, 10), 3)),
            may_actual
        );
        // third price added
        assert_eq!(
            Some(PriceAccumulator::test_new(price(6, 23), 4)),
            may_actual.and_then(|actual| actual.try_add(price(2, 1)))
        );
    }

    #[test]
    fn add_overflow_c() {
        let p1 = price(u128::from(u64::MAX) + 2, 1);
        let p2 = price(u128::from(u64::MAX) + 4, 1);

        let builder = PriceAccumulator::init_with(p1);

        let res = builder.try_add(p2);
        assert_eq!(None, res);
    }

    #[test]
    fn add_overflow_quotec() {
        let p1 = price(1, Amount::MAX / 2);
        let p2 = price(1, (Amount::MAX / 2) + 2);

        let builder = PriceAccumulator::init_with(p1);

        let res = builder.try_add(p2);

        assert_eq!(None, res);
    }

    #[test]
    fn price_average() {
        let builder = PriceAccumulator::test_new(price(3, 4), 5);
        let exp = price(15, 4);
        assert_eq!(Some(exp), builder.average());
    }

    #[test]
    fn price_average_max_values() {
        let builder = PriceAccumulator::test_new(price(Amount::MAX - 1, Amount::MAX), Unit::MAX);
        // Amount::MAX / Unit::MAX = 79228162532711081671548469249
        let exp = price(Amount::MAX - 1, 79228162532711081671548469249);
        assert_eq!(Some(exp), builder.average());
    }

    #[test]
    fn count_overflow() {
        let builder = PriceAccumulator::test_new(price(1, 2), u32::MAX);
        assert_eq!(None, builder.try_add(price(3, 4)));
    }

    fn price(amount: Amount, quote_amount: Amount) -> Price<SuperGroupTestC2, SuperGroupTestC1> {
        price::total_of(coin::coin2(amount)).is(coin::coin1(quote_amount))
    }
}
