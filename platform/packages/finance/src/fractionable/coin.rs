use crate::{
    coin::{Amount, Coin},
    duration::Duration,
    fractionable::Fractionable,
    percent::Percent,
};

impl<C> Fractionable<Percent> for Coin<C> {
    type MaxRank = Amount;
}

impl<C> Fractionable<Duration> for Coin<C> {
    type MaxRank = Amount;
}

impl<C> Fractionable<Coin<C>> for Coin<C> {
    type MaxRank = Amount;
}
#[cfg(test)]
mod test {
    use currency::test::SuperGroupTestC1;

    use crate::{
        coin::{Amount, Coin},
        percent::Percent,
        ratio::Rational,
    };

    #[test]
    fn safe_mul() {
        use crate::fractionable::Fractionable;
        assert_eq!(
            Coin::<SuperGroupTestC1>::new(30),
            Coin::<SuperGroupTestC1>::new(3).safe_mul(&Percent::from_percent(1000))
        );

        assert_eq!(
            Coin::<SuperGroupTestC1>::new(1000),
            Fractionable::<u32>::safe_mul(
                Coin::<SuperGroupTestC1>::new(2),
                &Rational::new(1000u32, 2u32)
            )
        );

        assert_eq!(
            Coin::<SuperGroupTestC1>::new(2 * Amount::from(u32::MAX)),
            Fractionable::<u32>::safe_mul(
                Coin::<SuperGroupTestC1>::new(2),
                &Rational::new(u32::MAX, 1u32)
            )
        );
    }
}
