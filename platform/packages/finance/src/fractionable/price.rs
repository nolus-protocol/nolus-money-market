use std::marker::PhantomData;

use crate::{coin::Amount, percent::Units as PercentUnits, price::Price, ratio::Ratio};

use super::Fractionable;

impl<C, QuoteC> Fractionable<PercentUnits> for Price<C, QuoteC>
where
    C: 'static,
    QuoteC: 'static,
{
    fn safe_mul<F>(self, fraction: &F) -> Self
    where
        F: Ratio<PercentUnits>,
    {
        self.lossy_mul(&RatioUpcast(PhantomData, fraction))
    }
}

impl<C, QuoteC> Fractionable<usize> for Price<C, QuoteC>
where
    C: 'static,
    QuoteC: 'static,
{
    fn safe_mul<F>(self, fraction: &F) -> Self
    where
        F: Ratio<usize>,
    {
        self.lossy_mul(&RatioTryUpcast(fraction))
    }
}

struct RatioUpcast<'a, U, R>(PhantomData<U>, &'a R)
where
    R: Ratio<U>;
impl<U, R> Ratio<Amount> for RatioUpcast<'_, U, R>
where
    U: Into<Amount>,
    R: Ratio<U>,
{
    fn parts(&self) -> Amount {
        self.1.parts().into()
    }
    fn total(&self) -> Amount {
        self.1.total().into()
    }
}

struct RatioTryUpcast<'a, R>(&'a R)
where
    R: Ratio<usize>;

const EXPECT_MSG: &str = "usize should convert into u128";

impl<R> Ratio<Amount> for RatioTryUpcast<'_, R>
where
    R: Ratio<usize>,
{
    fn parts(&self) -> Amount {
        self.0.parts().try_into().expect(EXPECT_MSG)
    }
    fn total(&self) -> Amount {
        self.0.total().try_into().expect(EXPECT_MSG)
    }
}

#[cfg(test)]
mod test {
    use currency::test::{SubGroupTestC10, SuperGroupTestC1};

    use crate::coin::{Amount, Coin};

    mod percent {
        use crate::fraction::Fraction;
        use crate::fractionable::price::test::{c, q};
        use crate::{percent::Percent, price};

        #[test]
        fn greater_than_one() {
            let price = price::total_of(c(1)).is(q(1000));
            let permille = Percent::from_permille(1);
            assert_eq!(permille.of(price), price::total_of(c(1)).is(q(1)));
        }

        #[test]
        fn less_than_one() {
            let price = price::total_of(c(10)).is(q(1));
            let twenty_percents = Percent::from_percent(20);
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
