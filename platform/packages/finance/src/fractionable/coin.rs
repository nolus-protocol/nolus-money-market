use crate::{
    coin::{Amount, Coin},
    duration::Duration,
    percent::{Units as PercentUnits, bound::BoundPercent},
};

use super::Fractionable;

impl<const UPPER_BOUND: PercentUnits, C> Fractionable<BoundPercent<UPPER_BOUND>> for Coin<C> {
    type MaxRank = Amount;
}

impl<C> Fractionable<Duration> for Coin<C> {
    type MaxRank = Amount;
}

impl<C> Fractionable<Amount> for Coin<C> {
    type MaxRank = Amount;
}

#[cfg(test)]
mod test {
    use currency::test::SuperGroupTestC1;

    use crate::{
        coin::{Amount, Coin},
        fraction::Fraction,
        percent::{Percent, Percent100},
        ratio::Ratio,
    };

    #[test]
    fn checked_mul() {
        assert_eq!(
            Coin::<SuperGroupTestC1>::new(30),
            Percent100::from_percent(100).of(Coin::<SuperGroupTestC1>::new(30))
        );

        assert_eq!(
            Coin::<SuperGroupTestC1>::new(4),
            Ratio::new(
                Percent100::from_permille(2u32),
                Percent100::from_permille(3u32)
            )
            .of(Coin::<SuperGroupTestC1>::new(6))
        );

        assert_eq!(
            Coin::<SuperGroupTestC1>::new(Amount::from(u32::MAX - 1)),
            Ratio::new(
                Percent::from_permille(u32::MAX - 1),
                Percent::from_permille(u32::MAX)
            )
            .of(Coin::<SuperGroupTestC1>::new(Amount::from(u32::MAX))),
        );
    }
}
