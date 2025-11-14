use std::marker::PhantomData;

use crate::{
    coin::Amount, fractionable::FractionableLegacy, percent::Units as PercentUnits, price::Price,
    ratio::RatioLegacy,
};

impl<C, QuoteC> FractionableLegacy<PercentUnits> for Price<C, QuoteC>
where
    C: 'static,
    QuoteC: 'static,
{
    fn safe_mul<F>(self, fraction: &F) -> Self
    where
        F: RatioLegacy<PercentUnits>,
    {
        self.lossy_mul(&RatioUpcast(PhantomData, fraction))
    }
}

// Used only for average price calculation
impl<C, QuoteC> FractionableLegacy<u128> for Price<C, QuoteC>
where
    C: 'static,
    QuoteC: 'static,
{
    fn safe_mul<F>(self, fraction: &F) -> Self
    where
        F: RatioLegacy<u128>,
    {
        self.lossy_mul(fraction)
    }
}

struct RatioUpcast<'a, U, R>(PhantomData<U>, &'a R)
where
    R: RatioLegacy<U>;

impl<U, R> RatioLegacy<Amount> for RatioUpcast<'_, U, R>
where
    U: Into<Amount>,
    R: RatioLegacy<U>,
{
    fn parts(&self) -> Amount {
        self.1.parts().into()
    }
    fn total(&self) -> Amount {
        self.1.total().into()
    }
}

#[cfg(test)]
mod test {
    use currency::test::{SubGroupTestC10, SuperGroupTestC1};

    use crate::coin::{Amount, Coin};

    mod percent {
        use crate::fraction::FractionLegacy;
        use crate::{percent::Percent100, price};

        #[test]
        fn greater_than_one() {
            let price = price::total_of(super::c(1)).is(super::q(1000));
            let permille = Percent100::from_permille(1);
            assert_eq!(
                permille.of(price),
                price::total_of(super::c(1)).is(super::q(1))
            );
        }

        #[test]
        fn less_than_one() {
            let price = price::total_of(super::c(10)).is(super::q(1));
            let twenty_percents = Percent100::from_percent(20);
            assert_eq!(
                twenty_percents.of(price),
                price::total_of(super::c(50)).is(super::q(1))
            );
        }
    }
    mod u128_ratio {
        use currency::test::{SubGroupTestC10, SuperGroupTestC1};

        use crate::{
            coin::{Amount, Coin},
            price,
            ratio::SimpleFraction,
            rational::RationalLegacy,
        };

        #[test]
        fn greater_than_one() {
            test_impl(super::c(1), super::q(999), 2, 3, super::c(1), super::q(666));
            test_impl(
                super::c(2),
                super::q(Amount::MAX),
                2,
                1,
                super::c(1),
                super::q(Amount::MAX),
            );
            // follow with rounding
            {
                let exp_q = 255211775190703847597530955573826158591; // (Amount::MAX * 3) >> 2;
                let exp_c = (2 * 4) >> 2;
                test_impl(
                    super::c(2),
                    super::q(Amount::MAX),
                    3,
                    4,
                    super::c(exp_c),
                    super::q(exp_q),
                );
            }
            {
                let exp_q = 212676479325586539664609129644855132159; // (Amount::MAX * 5) >> 3;
                let exp_c = (2 * 4) >> 3;
                test_impl(
                    super::c(2),
                    super::q(Amount::MAX),
                    5,
                    4,
                    super::c(exp_c),
                    super::q(exp_q),
                );
            }
        }

        #[test]
        fn less_than_one() {
            test_impl(super::c(150), super::q(1), 3, 2, super::c(100), super::q(1));
            test_impl(
                super::c(Amount::MAX),
                super::q(6),
                2,
                3,
                super::c(Amount::MAX),
                super::q(4),
            );
            // follow with rounding
            let exp_c = 191408831393027885698148216680369618943; // (Amount::MAX * 9) >> 4;
            let exp_q = (8 * 4) >> 4;
            test_impl(
                super::c(Amount::MAX),
                super::q(8),
                4,
                9,
                super::c(exp_c),
                super::q(exp_q),
            );
        }

        #[test]
        #[should_panic = "price overflow"]
        fn overflow() {
            test_impl(
                super::c(2),
                super::q(Amount::MAX),
                9,
                4,
                super::c(1),
                super::q(Amount::MAX),
            );
        }

        #[track_caller]
        fn test_impl(
            amount1: Coin<SubGroupTestC10>,
            quote1: Coin<SuperGroupTestC1>,
            nominator: u128,
            denominator: u128,
            amount_exp: Coin<SubGroupTestC10>,
            quote_exp: Coin<SuperGroupTestC1>,
        ) {
            let price = price::total_of(amount1).is(quote1);
            let ratio = SimpleFraction::new(nominator, denominator);
            assert_eq!(
                RationalLegacy::<u128>::of(&ratio, price).unwrap(),
                price::total_of(amount_exp).is(quote_exp)
            );
        }
    }
    fn c(a: Amount) -> Coin<SubGroupTestC10> {
        Coin::new(a)
    }

    fn q(a: Amount) -> Coin<SuperGroupTestC1> {
        Coin::new(a)
    }
}
