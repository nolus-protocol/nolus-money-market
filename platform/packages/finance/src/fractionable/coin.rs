use bnum::types::U256;

use crate::{
    coin::{Amount, Coin},
    duration::Duration,
    fractionable::{Fractionable, ToPrimitive, TryFromPrimitive},
    percent::Units as PercentUnits,
};

impl<C> Fractionable<Duration> for Coin<C> {
    type HigherPrimitive = U256;
}

impl<C, QuoteC> Fractionable<Coin<C>> for Coin<QuoteC> {
    type HigherPrimitive = U256;
}

impl<C> Fractionable<PercentUnits> for Coin<C> {
    type HigherPrimitive = U256;
}

impl<C> ToPrimitive<U256> for Coin<C> {
    fn into_primitive(self) -> U256 {
        self.amount().into()
    }
}

impl<C> TryFromPrimitive<U256> for Coin<C> {
    fn try_from_primitive(primitive: U256) -> Option<Self> {
        Amount::try_from(primitive)
            .ok()
            .map(|amount| Coin::<C>::new(amount))
    }
}

#[cfg(test)]
mod test {
    use currency::test::SuperGroupTestC1;

    use crate::{
        coin::{Amount, Coin},
        fraction::Fraction,
        percent::Percent100,
        ratio::SimpleFraction,
        rational::Rational,
    };

    #[test]
    fn of() {
        assert_eq!(
            Coin::<SuperGroupTestC1>::new(30),
            Percent100::from_percent(100).of(Coin::<SuperGroupTestC1>::new(30))
        );

        assert_eq!(
            Coin::<SuperGroupTestC1>::new(4),
            SimpleFraction::new(2u32, 3u32)
                .of(Coin::<SuperGroupTestC1>::new(6))
                .unwrap()
        );

        assert_eq!(
            Coin::<SuperGroupTestC1>::new(Amount::from(u32::MAX - 1)),
            SimpleFraction::new(u32::MAX - 1, u32::MAX)
                .of(Coin::<SuperGroupTestC1>::new(Amount::from(u32::MAX)))
                .unwrap()
        );
    }
}
