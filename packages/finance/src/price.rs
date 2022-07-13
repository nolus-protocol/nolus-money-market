use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{coin::Coin, currency::Currency, fraction::Fraction, ratio::Rational};

pub fn ratio<From>(from: Coin<From>) -> RatioBuilder<From>
where
    From: Currency,
{
    RatioBuilder(from)
}

pub struct RatioBuilder<From>(Coin<From>)
where
    From: Currency;

impl<From> RatioBuilder<From>
where
    From: Currency,
{
    // TODO remove the configuration attribute once start using the method in production code
    #[cfg(test)]
    fn to<To>(self, to: Coin<To>) -> ConversionRatio<From, To>
    where
        To: Currency,
    {
        ConversionRatio {
            amount1: self.0,
            amount2: to,
        }
    }
}

#[derive(Clone, Copy, Serialize, Deserialize, JsonSchema)]
pub struct ConversionRatio<C1, C2>
where
    C1: Currency,
    C2: Currency,
{
    amount1: Coin<C1>,
    amount2: Coin<C2>,
}

impl<C1, C2> ConversionRatio<C1, C2>
where
    C1: Currency,
    C2: Currency,
{
    pub fn inv(self) -> ConversionRatio<C2, C1> {
        ConversionRatio {
            amount1: self.amount2,
            amount2: self.amount1,
        }
    }
}

pub fn total<From, To>(of: Coin<From>, ratio: ConversionRatio<From, To>) -> Coin<To>
where
    From: Currency,
    To: Currency,
{
    let ratio_impl = Rational::new(of, ratio.amount1);
    <Rational<Coin<From>> as Fraction<Coin<From>>>::of(&ratio_impl, ratio.amount2)
}

#[cfg(test)]
mod test {
    use crate::{
        coin::Coin,
        currency::{Nls, Usdc},
    };

    type BaseCoin = Coin<Usdc>;
    type OtherCoin = Coin<Nls>;

    #[test]
    fn total() {
        let amount_base = 647;
        let amount_other = 48;
        let price = super::ratio(OtherCoin::new(amount_other)).to(BaseCoin::new(amount_base));
        let factor = 17;
        let coin_base = BaseCoin::new(amount_base * factor);
        let coin_other = OtherCoin::new(amount_other * factor);

        assert_eq!(coin_base, super::total(coin_other, price));
        assert_eq!(coin_other, super::total(coin_base, price.inv()));
    }

    #[test]
    fn total_rounding() {
        let amount_base = 647;
        let amount_other = 48;
        let price = super::ratio(OtherCoin::new(amount_other)).to(BaseCoin::new(amount_base));
        let coin_base = BaseCoin::new(633);

        // 47 * 647 / 48 -> 633.5208333333334
        let coin_other_in = OtherCoin::new(47);
        assert_eq!(coin_base, super::total(coin_other_in, price));

        // 633 * 48 / 647 -> 46.9613601236476
        let coin_other_out = OtherCoin::new(46);
        assert_eq!(coin_other_out, super::total(coin_base, price.inv()));
    }
}
