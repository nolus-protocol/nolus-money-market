use std::{fmt::Debug, marker::PhantomData};

use crate::{
    coin::Amount,
    percent::Units as PercentUnits,
    price::Price,
    ratio::{Ratio, Rational},
    zero::Zero,
};

use super::Fractionable;

impl<C, QuoteC> Fractionable<PercentUnits> for Price<C, QuoteC>
where
    C: 'static,
    QuoteC: 'static,
{
    fn safe_mul(self, fraction: &Ratio<PercentUnits>) -> Self {
        self.lossy_mul(
            &RatioUpcast(
                PhantomData,
                &Rational::new(fraction.parts(), fraction.total()),
            )
            .into(),
        )
    }
}

struct RatioUpcast<'a, U>(PhantomData<U>, &'a Rational<U>);

impl<'a, U> From<RatioUpcast<'a, U>> for Rational<Amount>
where
    U: Copy + Debug + Into<Amount> + Ord + Zero,
{
    fn from(upcast: RatioUpcast<'a, U>) -> Self {
        Self::new(upcast.1.nominator().into(), upcast.1.denominator().into())
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
