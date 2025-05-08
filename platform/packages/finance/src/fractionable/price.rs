use crate::{
    fraction::Fraction, percent::Units as PercentUnits, price::Price, ratio::SimpleFraction,
};

use super::Fractionable;

impl<C, QuoteC> Fractionable<PercentUnits> for Price<C, QuoteC>
where
    C: 'static,
    QuoteC: 'static,
{
    fn safe_mul<F>(self, fraction: &F) -> Self
    where
        F: Fraction<PercentUnits>,
    {
        let parts = fraction.parts().into();
        let total = fraction.total().into();

        self.lossy_mul(&SimpleFraction::new(parts, total))
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
            assert_eq!(
                Fraction::of(&permille, price),
                price::total_of(c(1)).is(q(1))
            );
        }

        #[test]
        fn less_than_one() {
            let price = price::total_of(c(10)).is(q(1));
            let twenty_percents = Percent100::from_percent(20);
            assert_eq!(
                Fraction::of(&twenty_percents, price),
                price::total_of(c(50)).is(q(1))
            );
        }
    }
    fn c(a: Amount) -> Coin<SubGroupTestC10> {
        Coin::<SubGroupTestC10>::from(a)
    }

    fn q(a: Amount) -> Coin<SuperGroupTestC1> {
        Coin::<SuperGroupTestC1>::from(a)
    }
}
