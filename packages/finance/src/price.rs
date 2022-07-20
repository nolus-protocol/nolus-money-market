use std::any::TypeId;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    coin::{Amount, Coin},
    currency::Currency,
    fraction::Fraction,
    fractionable::HigherRank,
    ratio::Rational,
};

pub fn total_of<C>(amount: Coin<C>) -> PriceBuilder<C>
where
    C: 'static + Currency,
{
    debug_assert!(!amount.is_zero());
    PriceBuilder(amount)
}

pub struct PriceBuilder<C>(Coin<C>)
where
    C: 'static + Currency;

impl<C> PriceBuilder<C>
where
    C: Currency,
{
    pub fn is<QuoteC>(self, to: Coin<QuoteC>) -> Price<C, QuoteC>
    where
        QuoteC: Currency,
    {
        debug_assert!(!to.is_zero());
        Price::new(self.0, to)
    }
}

/// Represents the price of a currency in a quote currency, ref: https://en.wikipedia.org/wiki/Currency_pair
///
/// The price is always kept in a canonical form of the underlying ratio. The simplifies equality and comparison operations.
/// For example, Price<EUR, USD> 1.15, generally represented as EURUSD or EUR/USD, means that one EUR is exchanged for 1.15 USD.
/// Both amounts a price if composed of should be non-zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Price<C, QuoteC>
where
    C: 'static + Currency,
    QuoteC: 'static + Currency,
{
    amount: Coin<C>,
    amount_quote: Coin<QuoteC>,
}

impl<C, QuoteC> Price<C, QuoteC>
where
    C: Currency,
    QuoteC: Currency,
{
    fn new(amount: Coin<C>, amount_quote: Coin<QuoteC>) -> Self {
        debug_assert_ne!(TypeId::of::<C>(), TypeId::of::<QuoteC>());

        let (amount_normalized, amount_quote_normalized) = amount.into_coprime_with(amount_quote);
        Self {
            amount: amount_normalized,
            amount_quote: amount_quote_normalized,
        }
    }

    pub fn inv(self) -> Price<QuoteC, C> {
        Price {
            amount: self.amount_quote,
            amount_quote: self.amount,
        }
    }
}

impl<C, QuoteC> PartialOrd for Price<C, QuoteC>
where
    C: Currency,
    QuoteC: Currency,
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        // a/b < c/d if and only if a * d < b * c
        // taking into account that Price is like amount_quote/amount
        type DoubleType = <Amount as HigherRank<Amount>>::Type;

        let a: DoubleType = self.amount_quote.into();
        let d: DoubleType = other.amount.into();

        let b: DoubleType = self.amount.into();
        let c: DoubleType = other.amount_quote.into();
        (a * d).partial_cmp(&(b * c))
    }
}

/// Calculates the amount of given coins in another currency, referred here as `quote currency`
///
/// For example, total(10 EUR, 1.01 EURUSD) = 10.1 USD
pub fn total<C, QuoteC>(of: Coin<C>, price: Price<C, QuoteC>) -> Coin<QuoteC>
where
    C: Currency,
    QuoteC: Currency,
{
    let ratio_impl = Rational::new(of, price.amount);
    <Rational<Coin<C>> as Fraction<Coin<C>>>::of(&ratio_impl, price.amount_quote)
}

#[cfg(test)]
mod test {
    use crate::{
        coin::{Amount, Coin as CoinT},
        currency::{Nls, Usdc},
        price::{self, Price},
    };

    type QuoteCoin = CoinT<Usdc>;
    type Coin = CoinT<Nls>;

    #[test]
    fn new_c16n() {
        let amount = 13;
        let amount_quote = 15;
        let factor = 32;
        assert_eq!(
            Price::new(Coin::new(amount), QuoteCoin::new(amount_quote)),
            Price::new(
                Coin::new(amount * factor),
                QuoteCoin::new(amount_quote * factor)
            )
        );
    }

    #[test]
    fn eq() {
        let amount = 13;
        let amount_quote = 15;
        assert_ne!(
            Price::new(Coin::new(amount), QuoteCoin::new(amount_quote)),
            Price::new(Coin::new(amount), QuoteCoin::new(amount_quote + 1))
        );
        assert_ne!(
            Price::new(Coin::new(amount - 1), QuoteCoin::new(amount_quote)),
            Price::new(Coin::new(amount), QuoteCoin::new(amount_quote))
        );

        assert_eq!(
            Price::new(Coin::new(amount), QuoteCoin::new(amount_quote)),
            Price::new(Coin::new(amount), QuoteCoin::new(amount_quote))
        );

        assert_eq!(
            Price::new(QuoteCoin::new(amount_quote), Coin::new(amount)),
            Price::new(Coin::new(amount), QuoteCoin::new(amount_quote)).inv()
        );
    }

    #[test]
    fn ord() {
        ord_impl(13, 15);
    }

    #[test]
    fn ord_max() {
        ord_impl(Amount::MAX, 1);
        ord_impl(Amount::MAX, Amount::MAX - 2);
    }

    #[test]
    fn total() {
        let amount_quote = 647;
        let amount = 48;
        let price = price::total_of(amount.into()).is(amount_quote.into());
        let factor = 17;
        let coin_quote = QuoteCoin::new(amount_quote * factor);
        let coin = Coin::new(amount * factor);

        assert_eq!(coin_quote, super::total(coin, price));
        assert_eq!(coin, super::total(coin_quote, price.inv()));
    }

    #[test]
    fn total_rounding() {
        let amount_quote = 647;
        let amount = 48;
        let price = super::total_of(amount.into()).is(amount_quote.into());
        let coin_quote = QuoteCoin::new(633);

        // 47 * 647 / 48 -> 633.5208333333334
        let coin_in = Coin::new(47);
        assert_eq!(coin_quote, super::total(coin_in, price));

        // 633 * 48 / 647 -> 46.9613601236476
        let coin_out = Coin::new(46);
        assert_eq!(coin_out, super::total(coin_quote, price.inv()));
    }

    #[test]
    fn total_max() {
        total_max_impl(1, 1, Amount::MAX, Amount::MAX);
        total_max_impl(100, 100, Amount::MAX, Amount::MAX);
        total_max_impl(50, 100, Amount::MAX - 1, (Amount::MAX - 1) / 2);
    }

    #[test]
    #[should_panic]
    fn total_overflow() {
        let price = price::total_of::<Nls>(1.into()).is::<Usdc>((Amount::MAX / 2 + 1).into());
        super::total(2.into(), price);
    }

    fn ord_impl(amount: Amount, amount_quote: Amount) {
        let price1 = Price::new(amount.into(), QuoteCoin::new(amount_quote));
        let price2 = Price::new(amount.into(), QuoteCoin::new(amount_quote + 1));
        assert!(price1 < price2);

        let total1 = super::total(Coin::new(amount), price1);
        assert!(total1 < super::total(Coin::new(amount), price2));
        assert_eq!(QuoteCoin::new(amount_quote), total1);
    }

    fn total_max_impl(
        amount: Amount,
        price_amount: Amount,
        price_amount_quote: Amount,
        expected: Amount,
    ) {
        let price = price::total_of(price_amount.into()).is(price_amount_quote.into());
        let expected = QuoteCoin::new(expected);
        let input = Coin::new(amount);

        assert_eq!(expected, super::total(input, price));
        assert_eq!(input, super::total(expected, price.inv()));
    }
}
