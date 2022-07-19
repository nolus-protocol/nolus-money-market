use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{coin::Coin, currency::Currency, fraction::Fraction, ratio::Rational};

pub fn total_of<C>(amount: Coin<C>) -> PriceBuilder<C>
where
    C: Currency,
{
    PriceBuilder(amount)
}

pub struct PriceBuilder<C>(Coin<C>)
where
    C: Currency;

impl<C> PriceBuilder<C>
where
    C: Currency,
{
    pub fn is<QuoteC>(self, to: Coin<QuoteC>) -> Price<C, QuoteC>
    where
        QuoteC: Currency,
    {
        Price {
            amount: self.0,
            amount_quote: to,
        }
    }
}

/// Represents the price of a currency in a quote currency
///
/// Ref: https://en.wikipedia.org/wiki/Currency_pair
///
/// For example, Price<EUR, USD> 1.15, generally represented as EURUSD or EUR/USD, means that one EUR is exchanged for 1.15 USD.
#[derive(Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Debug)]
pub struct Price<C, QuoteC>
where
    C: Currency,
    QuoteC: Currency,
{
    amount: Coin<C>,
    amount_quote: Coin<QuoteC>,
}

impl<C, QuoteC> Price<C, QuoteC>
where
    C: Currency,
    QuoteC: Currency,
{
    pub fn inv(self) -> Price<QuoteC, C> {
        Price {
            amount: self.amount_quote,
            amount_quote: self.amount,
        }
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
        coin::Coin as CoinT,
        currency::{Nls, Usdc},
        price,
    };

    type QuoteCoin = CoinT<Usdc>;
    type Coin = CoinT<Nls>;

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
}
