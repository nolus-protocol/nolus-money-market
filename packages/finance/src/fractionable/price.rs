use crate::{
    coin::Amount, currency::Currency, percent::Units as PercentUnits, price::Price, ratio::Ratio,
};

use super::Fractionable;

impl<C, QuoteC> Fractionable<PercentUnits> for Price<C, QuoteC>
where
    C: Currency,
    QuoteC: Currency,
{
    fn safe_mul<F>(self, fraction: &F) -> Self
    where
        F: Ratio<PercentUnits>,
    {
        struct RatioProxy<'a, R>(&'a R)
        where
            R: Ratio<PercentUnits>;

        impl<'a, R> Ratio<Amount> for RatioProxy<'a, R>
        where
            R: Ratio<PercentUnits>,
        {
            fn parts(&self) -> Amount {
                self.0.parts().into()
            }
            fn total(&self) -> Amount {
                self.0.total().into()
            }
        }
        self.lossy_mul(&RatioProxy(fraction))
    }
}

#[cfg(test)]
mod test {
    use crate::fraction::Fraction;
    use crate::{
        coin::{Amount, Coin},
        percent::Percent,
        price,
        test::currency::{Dai, Usdc},
    };

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

    fn c(a: Amount) -> Coin<Dai> {
        Coin::<Dai>::from(a)
    }

    fn q(a: Amount) -> Coin<Usdc> {
        Coin::<Usdc>::from(a)
    }
}
