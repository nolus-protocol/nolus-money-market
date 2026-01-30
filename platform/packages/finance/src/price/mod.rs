use std::fmt::{Debug, Formatter};

use serde::{Deserialize, Serialize};

use crate::{
    coin::{Amount, Coin},
    error::{Error, Result},
    fraction::Coprime,
    fractionable::IntoDoublePrimitive,
    ratio::SimpleFraction,
    rational::Rational,
};

mod arithmetics;
pub mod base;
pub mod dto;

pub const fn total_of<C>(amount: Coin<C>) -> PriceBuilder<C> {
    PriceBuilder(amount)
}

pub struct PriceBuilder<C>(Coin<C>)
where
    C: 'static;

impl<C> PriceBuilder<C>
where
    C: 'static,
{
    pub fn is<QuoteC>(self, to: Coin<QuoteC>) -> Price<C, QuoteC>
    where
        QuoteC: 'static,
    {
        Price::new(self.0, to)
    }
}

/// Represents the price of a currency in a quote currency, ref: <https://en.wikipedia.org/wiki/Currency_pair>
///
/// The price is always kept in a canonical form of the underlying ratio. The simplifies equality and comparison operations.
/// For example, Price<EUR, USD> 1.15, generally represented as EURUSD or EUR/USD, means that one EUR is exchanged for 1.15 USD.
/// Both amounts a price is composed of should be non-zero.
///
/// Not designed to be used in public APIs
#[derive(Serialize, Deserialize)]
#[serde(bound(serialize = "", deserialize = ""))]
pub struct Price<C, QuoteC> {
    amount: Coin<C>,
    amount_quote: Coin<QuoteC>,
}

impl<C, QuoteC> Price<C, QuoteC>
where
    C: 'static,
    QuoteC: 'static,
{
    /// Constructor intended to be used when the preconditions have already been met,
    /// for example when converting from another Price family instance, e.g. PriceDTO
    #[track_caller]
    fn new(amount: Coin<C>, amount_quote: Coin<QuoteC>) -> Self {
        debug_assert_eq!(Ok(()), Self::precondition_check(amount, amount_quote));

        let res = Self::new_inner(amount, amount_quote);

        debug_assert_eq!(Ok(()), res.invariant_held());

        res
    }

    /// Constructor intended to be used with non-validated input,
    /// for example when deserializing from an user request
    #[track_caller]
    fn try_new(amount: Coin<C>, amount_quote: Coin<QuoteC>) -> Result<Self> {
        Self::precondition_check(amount, amount_quote)
            .map(|()| Self::new_inner(amount, amount_quote))
            .and_then(|may_price| may_price.invariant_held().map(|()| may_price))
    }

    fn new_inner(amount: Coin<C>, amount_quote: Coin<QuoteC>) -> Self {
        let (amount_normalized, amount_quote_normalized): (Coin<C>, Coin<QuoteC>) =
            amount.to_coprime_with(amount_quote);

        Self {
            amount: amount_normalized,
            amount_quote: amount_quote_normalized,
        }
    }

    /// Returns a new [`Price`] which represents identity mapped, one to one, currency pair.
    pub const fn identity() -> Self {
        Self {
            amount: Coin::new(1),
            amount_quote: Coin::new(1),
        }
    }

    pub fn inv(self) -> Price<QuoteC, C> {
        Price {
            amount: self.amount_quote,
            amount_quote: self.amount,
        }
    }

    fn precondition_check(amount: Coin<C>, amount_quote: Coin<QuoteC>) -> Result<()> {
        Self::check(!amount.is_zero(), "The amount should not be zero").and(Self::check(
            !amount_quote.is_zero(),
            "The quote amount should not be zero",
        ))
    }

    fn invariant_held(&self) -> Result<()> {
        Self::precondition_check(self.amount, self.amount_quote).and(Self::check(
            Amount::from(self.amount) == Amount::from(self.amount_quote)
                || !currency::equal::<C, QuoteC>(),
            "The price should be equal to the identity if the currencies match",
        ))
    }

    fn check(invariant: bool, msg: &str) -> Result<()> {
        Error::broken_invariant_if::<Self>(!invariant, msg)
    }
}

impl<C, QuoteC> Clone for Price<C, QuoteC>
where
    C: 'static,
    QuoteC: 'static,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<C, QuoteC> Copy for Price<C, QuoteC>
where
    C: 'static,
    QuoteC: 'static,
{
}

impl<C, QuoteC> Debug for Price<C, QuoteC> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Price")
            .field("amount", &self.amount)
            .field(" amount_quote", &self.amount_quote)
            .finish()
    }
}

impl<C, QuoteC> Eq for Price<C, QuoteC>
where
    C: 'static,
    QuoteC: 'static,
{
}

impl<C, QuoteC> PartialEq for Price<C, QuoteC>
where
    C: 'static,
    QuoteC: 'static,
{
    fn eq(&self, other: &Self) -> bool {
        self.amount == other.amount && self.amount_quote == other.amount_quote
    }
}

impl<C, QuoteC> Ord for Price<C, QuoteC>
where
    C: 'static,
    QuoteC: 'static,
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // a/b < c/d if and only if a * d < b * c
        // Please note that Price(amount, amount_quote) is like Ratio(amount_quote / amount).

        let a = self.amount_quote.into_double();
        let d = other.amount.into_double();

        let b = self.amount.into_double();
        let c = other.amount_quote.into_double();
        (a * d).cmp(&(b * c))
    }
}

impl<C, QuoteC> PartialOrd for Price<C, QuoteC>
where
    C: 'static,
    QuoteC: 'static,
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Calculates the amount of given coins in another currency, referred here as `quote currency`.
/// Returns `None` if an overflow occurs during the calculation.
///
/// For example, total(10 EUR, 1.01 EURUSD) = 10.1 USD
pub fn total<C, QuoteC>(of: Coin<C>, price: Price<C, QuoteC>) -> Option<Coin<QuoteC>> {
    SimpleFraction::new(of, price.amount).of(price.amount_quote)
}

#[cfg(test)]
mod test {
    use currency::test::{SubGroupTestC10, SuperGroupTestC1, SuperGroupTestC2};

    use crate::{
        coin::{Amount, Coin as CoinT},
        price::{self, Price},
        test::coin,
    };

    pub(super) type QuoteQuoteCoin = CoinT<SubGroupTestC10>;
    pub(super) type QuoteCoin = CoinT<SuperGroupTestC1>;
    pub(super) type Coin = CoinT<SuperGroupTestC2>;

    #[test]
    fn new_c16n() {
        let amount = 13;
        let amount_quote = 15;
        let factor = 32;
        assert_eq!(
            price(coin::coin2(amount), coin::coin1(amount_quote)),
            price(
                coin::coin2(amount * factor),
                coin::coin1(amount_quote * factor)
            )
        );
    }

    #[test]
    fn eq() {
        let amount = 13;
        let amount_quote = 15;
        assert_ne!(
            price(coin::coin2(amount), coin::coin1(amount_quote)),
            price(coin::coin2(amount), coin::coin1(amount_quote + 1))
        );
        assert_ne!(
            price(coin::coin2(amount - 1), coin::coin1(amount_quote)),
            price(coin::coin2(amount), coin::coin1(amount_quote))
        );

        assert_eq!(
            price(coin::coin2(amount), coin::coin1(amount_quote)),
            price(coin::coin2(amount), coin::coin1(amount_quote))
        );

        assert_eq!(
            Price::new(coin::coin1(amount_quote), coin::coin2(amount)),
            price(coin::coin2(amount), coin::coin1(amount_quote)).inv()
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
        let price = price::total_of(coin::coin2(amount)).is(coin::coin1(amount_quote));
        let factor = 17;
        let coin_quote = coin::coin1(amount_quote * factor);
        let coin = coin::coin2(amount * factor);

        assert_eq!(coin_quote, calc_total(coin, price));
        assert_eq!(coin, super::total(coin_quote, price.inv()).unwrap());
    }

    #[test]
    fn total_rounding() {
        let amount_quote = 647;
        let amount = 48;
        let price = super::total_of(coin::coin2(amount)).is(coin::coin1(amount_quote));
        let coin_quote = coin::coin1(633);

        // 47 * 647 / 48 -> 633.5208333333334
        let coin_in = coin::coin2(47);
        assert_eq!(coin_quote, calc_total(coin_in, price));

        // 633 * 48 / 647 -> 46.9613601236476
        let coin_out = coin::coin2(46);
        assert_eq!(coin_out, super::total(coin_quote, price.inv()).unwrap());
    }

    #[test]
    fn total_max() {
        total_max_impl(1, 1, Amount::MAX, Amount::MAX);
        total_max_impl(100, 100, Amount::MAX, Amount::MAX);
        total_max_impl(50, 100, Amount::MAX - 1, (Amount::MAX - 1) / 2);
    }

    #[test]
    fn total_overflow() {
        let price = price::total_of(coin::coin2(1)).is(coin::coin1(Amount::MAX / 2 + 1));
        assert!(super::total(coin::coin2(2), price).is_none());
    }

    pub(super) fn price(
        amount: Coin,
        amount_quote: QuoteCoin,
    ) -> Price<SuperGroupTestC2, SuperGroupTestC1> {
        Price::new(amount, amount_quote)
    }

    fn calc_total(coin: Coin, price: Price<SuperGroupTestC2, SuperGroupTestC1>) -> QuoteCoin {
        super::total(coin, price).unwrap()
    }

    fn ord_impl(amount: Amount, amount_quote: Amount) {
        let price1 = price(coin::coin2(amount), coin::coin1(amount_quote));
        let price2 = price(coin::coin2(amount), coin::coin1(amount_quote + 1));
        assert!(price1 < price2);

        let total1 = calc_total(coin::coin2(amount), price1);
        assert!(total1 < calc_total(coin::coin2(amount), price2));
        assert_eq!(coin::coin1(amount_quote), total1);
    }

    fn total_max_impl(
        amount: Amount,
        price_amount: Amount,
        price_amount_quote: Amount,
        expected: Amount,
    ) {
        let price = price::total_of(coin::coin2(price_amount)).is(coin::coin1(price_amount_quote));
        let expected = coin::coin1(expected);
        let input = coin::coin2(amount);

        assert_eq!(expected, calc_total(input, price));
        assert_eq!(input, super::total(expected, price.inv()).unwrap());
    }
}

#[cfg(test)]
mod test_invariant {
    use currency::Currency;

    use crate::{coin::Coin, price::Price, test::coin};

    #[test]
    #[should_panic = "zero"]
    fn base_zero() {
        new_invalid(coin::coin1(0), coin::coin2(5));
    }

    #[test]
    #[should_panic = "zero"]
    fn quote_zero() {
        new_invalid(coin::coin1(10), coin::coin2(0));
    }

    #[test]
    #[should_panic = "should be equal to the identity if the currencies match"]
    fn currencies_match() {
        new_invalid(coin::coin2(4), coin::coin2(5));
    }

    #[test]
    fn currencies_match_ok() {
        assert_eq!(
            Price::identity(),
            Price::new(coin::coin2(4), coin::coin2(4))
        );
    }

    fn new_invalid<C, QuoteC>(base: Coin<C>, quote: Coin<QuoteC>)
    where
        C: Currency,
        QuoteC: Currency,
    {
        let _p = Price::new(base, quote);
        #[cfg(not(debug_assertions))]
        {
            _p.invariant_held().expect("should have returned an error");
        }
    }
}
