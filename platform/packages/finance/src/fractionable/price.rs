use bnum::types::U256;

use crate::{
    fraction::Unit as FractionUnit,
    fractionable::{Fractionable, ToPrimitive, TryFromPrimitive},
    percent::Units as PercentUnits,
    price::Price,
    ratio::SimpleFraction,
    zero::Zero,
};

impl<C, QuoteC> Fractionable<PercentUnits> for Price<C, QuoteC>
where
    C: 'static,
    QuoteC: 'static,
{
    type HigherPrimitive = SimpleFraction<U256>;
}

impl FractionUnit for U256 {}

impl<C, QuoteC> ToPrimitive<SimpleFraction<U256>> for Price<C, QuoteC>
where
    C: 'static,
    QuoteC: 'static,
{
    fn into_primitive(self) -> SimpleFraction<U256> {
        self.to_fraction()
    }
}

impl<C, QuoteC> TryFromPrimitive<SimpleFraction<U256>> for Price<C, QuoteC>
where
    C: 'static,
    QuoteC: 'static,
{
    fn try_from_primitive(primitive: SimpleFraction<U256>) -> Option<Self> {
        Self::try_from_fraction(primitive)
    }
}

impl Zero for U256 {
    const ZERO: Self = Self::ZERO;
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
