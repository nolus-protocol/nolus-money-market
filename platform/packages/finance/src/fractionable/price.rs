use crate::{
    coin::Amount,
    percent::{Units as PercentUnits, bound::BoundPercent},
    price::Price,
    ratio::SimpleFraction,
};

use super::Fractionable;

impl<const UPPER_BOUND: PercentUnits, C, QuoteC> Fractionable<BoundPercent<UPPER_BOUND>>
    for Price<C, QuoteC>
where
    C: 'static,
    QuoteC: 'static,
{
    type MaxRank = SimpleFraction<Amount>;
}

impl From<PercentUnits> for SimpleFraction<Amount> {
    fn from(value: PercentUnits) -> Self {
        let nominator = Amount::from(value);
        let denominator = Amount::from(1u128);

        Self::new(nominator, denominator)
    }
}

#[cfg(test)]
mod test {
    use currency::test::{SubGroupTestC10, SuperGroupTestC1};

    use crate::coin::{Amount, Coin};

    mod percent {
        use crate::fraction::Fraction;
        use crate::fractionable::price::test::{c, q};
        use crate::{percent::Percent100, price};

        #[test]
        fn greater_than_one() {
            let price = price::total_of(c(1)).is(q(1000));
            let permille = Percent100::from_permille(1);
            assert_eq!(permille.of(price), price::total_of(c(1)).is(q(1)));
        }

        #[test]
        fn less_than_one() {
            let price = price::total_of(c(10)).is(q(1));
            let twenty_percents = Percent100::from_percent(20);
            assert_eq!(twenty_percents.of(price), price::total_of(c(50)).is(q(1)));
        }
    }
    fn c(a: Amount) -> Coin<SubGroupTestC10> {
        Coin::<SubGroupTestC10>::from(a)
    }

    fn q(a: Amount) -> Coin<SuperGroupTestC1> {
        Coin::<SuperGroupTestC1>::from(a)
    }
}
