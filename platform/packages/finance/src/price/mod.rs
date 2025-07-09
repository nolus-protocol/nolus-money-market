use std::{
    convert::Infallible,
    fmt::{Debug, Display, Formatter, Result as FmtResult},
    ops::{Add, AddAssign, Mul},
};

use serde::{Deserialize, Serialize};

use crate::{
    coin::{Amount, Coin},
    error::{Error, Result as FinanceResult},
    ratio::SimpleFraction,
    rational::Rational,
    traits::{CheckedAdd, CheckedMul},
};

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
#[derive(Debug, Serialize, Deserialize)]
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

    /// Contructor intended to be used with non-validated input,
    /// for example when deserializing from an user request
    #[track_caller]
    fn try_new(amount: Coin<C>, amount_quote: Coin<QuoteC>) -> FinanceResult<Self> {
        Self::precondition_check(amount, amount_quote)
            .map(|()| Self::new_inner(amount, amount_quote))
            .and_then(|may_price| may_price.invariant_held().map(|()| may_price))
    }

    fn new_inner(amount: Coin<C>, amount_quote: Coin<QuoteC>) -> Self {
        let (amount_normalized, amount_quote_normalized): (Coin<C>, Coin<QuoteC>) =
            amount.into_coprime_with(amount_quote);

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

    /// Price(amount, amount_quote) * Ratio(nominator / denominator) = Price(amount * denominator, amount_quote * nominator)
    /// where the pairs (amount, nominator) and (amount_quote, denominator) are transformed into co-prime numbers.
    /// Please note that Price(amount, amount_quote) is like Ratio(amount_quote / amount).
    pub(crate) fn lossy_mul(self, rhs: SimpleFraction<Amount>) -> Self {
        let product = SimpleFraction::from(self)
            .checked_mul(rhs)
            .expect("price overflow during multiplication");
        product
            .try_into()
            .expect("lossy_mul failed: can't convert back to Price")
    }

    pub fn inv(self) -> Price<QuoteC, C> {
        Price {
            amount: self.amount_quote,
            amount_quote: self.amount,
        }
    }

    fn precondition_check(amount: Coin<C>, amount_quote: Coin<QuoteC>) -> FinanceResult<()> {
        Self::check(!amount.is_zero(), "The amount should not be zero").and(Self::check(
            !amount_quote.is_zero(),
            "The quote amount should not be zero",
        ))
    }

    fn invariant_held(&self) -> FinanceResult<()> {
        Self::precondition_check(self.amount, self.amount_quote).and(Self::check(
            Amount::from(self.amount) == Amount::from(self.amount_quote)
                || !currency::equal::<C, QuoteC>(),
            "The price should be equal to the identity if the currencies match",
        ))
    }

    fn check(invariant: bool, msg: &str) -> FinanceResult<()> {
        Error::broken_invariant_if::<Self>(!invariant, msg)
    }

    fn checked_add(self, rhs: Self) -> Option<Self> {
        // taking into account that Price is like amount_quote/amount
        let lhs_rational = SimpleFraction::from(self);
        let rhs_rational = SimpleFraction::from(rhs);

        lhs_rational
            .checked_add(rhs_rational)
            .map(|result_rational| {
                Self::new(
                    result_rational.denominator().into(),
                    result_rational.nominator().into(),
                )
            })
    }

    /// Add two prices rounding each of them to 1.10-18, simmilarly to
    /// the precision provided by CosmWasm's ['Decimal'][sdk::cosmwasm_std::Decimal].
    ///
    /// TODO Implement a variable precision algorithm depending on the
    /// value of the prices. The rounding would be done by shifting to
    /// the right both amounts of the price with a bigger denominator
    /// until a * d + b * c and b * d do not overflow.
    fn lossy_add(self, rhs: Self) -> Option<Self> {
        let factor: Coin<C> = Coin::new(1_000_000_000_000_000_000); // 1*10^18

        total(factor, self)
            .and_then(|total_self| {
                total(factor, rhs).and_then(|total_rhs| total_self.checked_add(total_rhs))
            })
            .map(|factored_total| total_of(factor).is(factored_total))
    }
}

impl<C, QuoteC> Add<Price<C, QuoteC>> for Price<C, QuoteC>
where
    C: 'static,
    QuoteC: 'static,
{
    type Output = Price<C, QuoteC>;

    fn add(self, rhs: Price<C, QuoteC>) -> Self::Output {
        self.checked_add(rhs)
            .or_else(|| self.lossy_add(rhs))
            .expect("should not observe huge prices")
    }
}

impl<C, QuoteC> AddAssign<Price<C, QuoteC>> for Price<C, QuoteC>
where
    C: 'static,
    QuoteC: 'static,
{
    #[track_caller]
    fn add_assign(&mut self, rhs: Price<C, QuoteC>) {
        *self = self.add(rhs);
    }
}

impl<C, QuoteC> CheckedAdd for Price<C, QuoteC>
where
    C: 'static,
    QuoteC: 'static,
{
    type Output = Self;

    fn checked_add(self, rhs: Self) -> Option<Self::Output> {
        self.checked_add(rhs)
    }
}

// TODO for completeness implement the Sub and SubAssign counterparts

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

impl<C, QuoteC> Display for Price<C, QuoteC> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}/{}", self.amount, self.amount_quote)
    }
}

impl<C, QuoteC> Eq for Price<C, QuoteC>
where
    C: 'static,
    QuoteC: 'static,
{
}

impl<C, QuoteC> From<Price<C, QuoteC>> for SimpleFraction<Amount>
where
    C: 'static,
    QuoteC: 'static,
{
    fn from(price: Price<C, QuoteC>) -> Self {
        // Please note that Price(amount, amount_quote) is like SimpleFraction(amount_quote / amount).
        price.amount_quote.to_rational(price.amount)
    }
}

impl<C, QuoteC, QuoteQuoteC> Mul<Price<QuoteC, QuoteQuoteC>> for Price<C, QuoteC>
where
    C: 'static,
    QuoteC: 'static,
    QuoteQuoteC: 'static,
{
    type Output = Price<C, QuoteQuoteC>;

    #[track_caller]
    fn mul(self, rhs: Price<QuoteC, QuoteQuoteC>) -> Self::Output {
        // Price(a, b) * Price(c, d) = Price(a, d) * Rational(b / c)
        // Please note that Price(amount, amount_quote) is like Ratio(amount_quote / amount).

        Self::Output::new(self.amount, rhs.amount_quote).lossy_mul(SimpleFraction::new(
            self.amount_quote.into(),
            rhs.amount.into(),
        ))
    }
}

impl<C, QuoteC> Ord for Price<C, QuoteC>
where
    C: 'static,
    QuoteC: 'static,
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        SimpleFraction::from(*self).cmp(&SimpleFraction::from(*other))
    }
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

impl<C, QuoteC> PartialOrd for Price<C, QuoteC>
where
    C: 'static,
    QuoteC: 'static,
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<C, QuoteC> TryFrom<SimpleFraction<Amount>> for Price<C, QuoteC>
where
    C: 'static,
    QuoteC: 'static,
{
    type Error = Infallible;

    fn try_from(fraction: SimpleFraction<Amount>) -> Result<Self, Self::Error> {
        Ok(Price::new(
            fraction.denominator().into(),
            fraction.nominator().into(),
        ))
    }
}

/// Calculates the amount of given coins in another currency, referred here as `quote currency`
///
/// For example, total(10 EUR, 1.01 EURUSD) = 10.1 USD
pub fn total<C, QuoteC>(of: Coin<C>, price: Price<C, QuoteC>) -> Option<Coin<QuoteC>> {
    let ratio_impl: SimpleFraction<Amount> = SimpleFraction::new(of.into(), price.amount.into());
    ratio_impl.of(price.amount_quote)
}

#[cfg(test)]
mod test {
    use std::ops::{Add, AddAssign, Mul};

    use currency::test::{SubGroupTestC10, SuperGroupTestC1, SuperGroupTestC2};

    use crate::{
        coin::{Amount, Coin as CoinT},
        price::{self, Price},
        ratio::SimpleFraction,
        traits::{Scalar, Trim},
    };

    type QuoteQuoteCoin = CoinT<SubGroupTestC10>;
    type QuoteCoin = CoinT<SuperGroupTestC1>;
    type Coin = CoinT<SuperGroupTestC2>;

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
        let price = super::total_of(amount.into()).is(amount_quote.into());
        let factor = 17;
        let coin_quote = QuoteCoin::new(amount_quote * factor);
        let coin = Coin::new(amount * factor);

        assert_eq!(coin_quote, super::total(coin, price).unwrap());
        assert_eq!(coin, super::total(coin_quote, price.inv()).unwrap());
    }

    #[test]
    fn total_rounding() {
        let amount_quote: u128 = 647;
        let amount: u128 = 48;
        let price = super::total_of(amount.into()).is(amount_quote.into());
        let coin_quote = QuoteCoin::new(633);

        // 47 * 647 / 48 -> 633.5208333333334
        let coin_in = Coin::new(47);
        assert_eq!(coin_quote, super::total(coin_in, price).unwrap());

        // 633 * 48 / 647 -> 46.9613601236476
        let coin_out = Coin::new(46);
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
        let price = price::total_of::<SuperGroupTestC2>(Coin::new(1))
            .is::<SuperGroupTestC1>((Amount::MAX / 2 + 1).into());
        assert!(super::total(Coin::new(2), price).is_none());
    }

    #[test]
    fn add_no_round() {
        add_impl(c(1), q(2), c(5), q(10), c(1), q(4));
        add_impl(c(2), q(1), c(10), q(5), c(1), q(1));
        add_impl(c(2), q(3), c(10), q(14), c(10), q(29));
    }

    #[test]
    fn add_round() {
        add_impl(c(Amount::MAX), q(1), c(1), q(1), c(1), q(1));
    }

    #[test]
    fn lossy_add_no_round() {
        lossy_add_impl(c(1), q(2), c(5), q(10), c(1), q(4));
        lossy_add_impl(c(2), q(1), c(10), q(5), c(1), q(1));
        lossy_add_impl(c(2), q(3), c(10), q(14), c(10), q(29));
    }

    #[test]
    fn lossy_add_round() {
        // 1/3 + 2/7 = 13/21 that is 0.(619047)*...
        let amount_exp = 1_000_000_000_000_000_000;
        let quote_exp = 619_047_619_047_619_047;
        lossy_add_impl(c(3), q(1), c(7), q(2), c(amount_exp), q(quote_exp));
        lossy_add_impl(
            c(amount_exp),
            q(quote_exp),
            c(3),
            q(1),
            c(amount_exp),
            q(quote_exp + 333_333_333_333_333_333),
        );
        lossy_add_impl(
            c(amount_exp + 1),
            q(quote_exp),
            c(21),
            q(1),
            c(amount_exp / 5),
            q(133_333_333_333_333_333),
        );

        lossy_add_impl(c(amount_exp + 1), q(1), c(1), q(1), c(1), q(1));

        lossy_add_impl(c(Amount::MAX), q(1), c(1), q(1), c(1), q(1));
    }

    #[test]
    fn lossy_add_overflow() {
        // 2^128 / FACTOR (10^18) / 2^64 ~ 18.446744073709553
        let p1 = price::total_of(c(1)).is(q(u128::from(u64::MAX) * 19u128));
        let p2 = Price::identity();
        assert!(p1.lossy_add(p2).is_none());
    }

    #[test]
    fn lossy_mul_no_round() {
        lossy_mul_impl(c(1), q(2), q(2), qq(1), c(1), qq(1));
        lossy_mul_impl(c(2), q(3), q(18), qq(5), c(12), qq(5));
        lossy_mul_impl(c(7), q(3), q(11), qq(21), c(11), qq(9));
        lossy_mul_impl(c(7), q(3), q(11), qq(23), c(7 * 11), qq(3 * 23));

        let big_int = u128::MAX - 1;
        assert!(big_int % 3 != 0 && big_int % 11 != 0);
        lossy_mul_impl(c(big_int), q(3), q(11), qq(big_int), c(11), qq(3));

        assert_eq!(0, u128::MAX % 5);
        lossy_mul_impl(c(u128::MAX), q(2), q(3), qq(5), c(u128::MAX / 5 * 3), qq(2));
    }

    #[test]
    fn lossy_mul_with_trim() {
        let amount1 = c(u64::MAX as u128);
        let quote1 = q(u128::MAX - 1);
        let amount2 = q(1u128);
        let quote2 = qq(2u128);

        // Price1{u64::MAX as u128, u128::MAX -1} * Price2{1,2}  is calculated as
        // SimpleFraction{u128::MAX -1, u64::MAX as u128} * SimpleFraction{2, 1}.
        // The multiplication (u128::MAX - 1) * 2 will overflow and the algorythm will trim 2 bits from the u128::MAX -1.
        // The trimmed nominator is calculated as follows:
        // u128::MAX - 1  = 1111...1110
        // (u128::MAX - 1) >> 2  = 0011...1111
        // 0011...1111 * 2 = 0011...1111 << 1  = 0111...1110 = 2^127 - 2
        // The trimmed denominator is calculated as follows:
        // u64::MAX = 1111...1111
        // u64::MAX >> 2 = 0011...1111 = 2^62 - 1

        lossy_mul_impl(
            amount1,
            quote1,
            amount2,
            quote2,
            c(2u128.pow(62) - 1),
            qq(2u128.pow(127) - 2),
        );
    }

    #[test]
    #[should_panic = "overflow"]
    fn lossy_mul_overflow() {
        const SHIFTS: u8 = 23;
        let a1 = u128::MAX - 1;
        let q1 = 7;
        let a2: Amount = 1 << SHIFTS;
        let q2 = a2 / q1 - 1; // the aim is q1 * q2 < a2

        assert!(a1 % q1 != 0);
        assert!(a1 % q2 != 0);
        assert!(a2 % q1 != 0);
        assert!(a2 % q2 != 0);

        assert!(shift_product(a1, a2, SHIFTS) == 0 || shift_product(q1, q2, SHIFTS) == 0);
        let price1 = price::total_of(c(a1)).is(q(q1));
        let price2 = price::total_of(q(a2)).is(qq(q2));
        _ = price1.mul(price2);
    }

    fn c(a: Amount) -> Coin {
        Coin::new(a)
    }

    fn q(a: Amount) -> QuoteCoin {
        QuoteCoin::new(a)
    }

    fn qq(a: Amount) -> QuoteQuoteCoin {
        QuoteQuoteCoin::new(a)
    }

    fn ord_impl(amount: Amount, amount_quote: Amount) {
        let price1 = Price::new(amount.into(), QuoteCoin::new(amount_quote));
        let price2 = Price::new(amount.into(), QuoteCoin::new(amount_quote + 1));
        assert!(price1 < price2);

        let total1 = super::total(Coin::new(amount), price1).unwrap();
        assert!(total1 < super::total(Coin::new(amount), price2).unwrap());
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

        assert_eq!(expected, super::total(input, price).unwrap());
        assert_eq!(input, super::total(expected, price.inv()).unwrap());
    }

    fn add_impl(
        amount1: Coin,
        quote1: QuoteCoin,
        amount2: Coin,
        quote2: QuoteCoin,
        amount_exp: Coin,
        quote_exp: QuoteCoin,
    ) {
        let price1 = price::total_of(amount1).is(quote1);
        let price2 = price::total_of(amount2).is(quote2);
        let exp = price::total_of(amount_exp).is(quote_exp);
        assert_eq!(exp, price1.add(price2));
        assert!({
            price1.checked_add(price2).map_or_else(
                || Some(exp) == price1.lossy_add(price2),
                |v| v == price1.add(price2),
            )
        });
        assert!(Some(exp) == price1.lossy_add(price2));
        assert!(exp >= price1);
        assert!(exp >= price2);

        let mut price3 = price1;
        price3.add_assign(price2);
        assert_eq!(exp, price3);
    }

    #[track_caller]
    fn lossy_add_impl(
        amount1: Coin,
        quote1: QuoteCoin,
        amount2: Coin,
        quote2: QuoteCoin,
        amount_exp: Coin,
        quote_exp: QuoteCoin,
    ) {
        let price1 = price::total_of(amount1).is(quote1);
        let price2 = price::total_of(amount2).is(quote2);
        let exp = price::total_of(amount_exp).is(quote_exp);
        assert_eq!(Some(exp), price1.lossy_add(price2));
        assert!(exp <= price1.add(price2));
    }

    fn shift_product(lhs: Amount, rhs: Amount, shifts: u8) -> Amount {
        let (lhs_share, rhs_share) = calc_shares(lhs, rhs, shifts);
        let lhs_trimmed = lhs.trim(lhs_share);
        let rhs_trimmed = rhs.trim(rhs_share);

        lhs_trimmed
            .scale_up(rhs_trimmed.into_times())
            .expect("even trimmed values overflowed")
            .trim(shifts.into())
    }

    fn calc_shares(lhs: Amount, rhs: Amount, shifts: u8) -> (u32, u32) {
        let lhs_bits = Amount::BITS - lhs.leading_zeros();
        let rhs_bits = Amount::BITS - rhs.leading_zeros();
        let total_bits = lhs_bits + rhs_bits;
        let shifts = shifts as u32;

        let prod = shifts * lhs_bits;

        let lhs_share = if 2 * (prod % total_bits) < total_bits {
            prod / total_bits
        } else {
            prod / total_bits + 1
        };

        (lhs_share, shifts - lhs_share)
    }

    fn lossy_mul_impl(
        amount1: Coin,
        quote1: QuoteCoin,
        amount2: QuoteCoin,
        quote2: QuoteQuoteCoin,
        amount_exp: Coin,
        quote_exp: QuoteQuoteCoin,
    ) {
        let price1 = price::total_of(amount1).is(quote1);
        let price2 = price::total_of(amount2).is(quote2);
        let exp = price::total_of(amount_exp).is(quote_exp);

        assert_eq!(exp, price1.mul(price2));

        let price3 = price::total_of(amount1).is(quote2);
        let ratio = SimpleFraction::new(quote1.into(), amount2.into());
        assert_eq!(exp, price3.lossy_mul(ratio));
    }
}

#[cfg(test)]
mod test_invariant {
    use currency::{
        Currency,
        test::{SuperGroupTestC1, SuperGroupTestC2},
    };

    use crate::{coin::Coin, price::Price};

    #[test]
    #[should_panic = "zero"]
    fn base_zero() {
        new_invalid(
            Coin::<SuperGroupTestC1>::new(0),
            Coin::<SuperGroupTestC2>::new(5),
        );
    }

    #[test]
    #[should_panic = "zero"]
    fn quote_zero() {
        new_invalid(
            Coin::<SuperGroupTestC1>::new(10),
            Coin::<SuperGroupTestC2>::new(0),
        );
    }

    #[test]
    #[should_panic = "should be equal to the identity if the currencies match"]
    fn currencies_match() {
        new_invalid(
            Coin::<SuperGroupTestC2>::new(4),
            Coin::<SuperGroupTestC2>::new(5),
        );
    }

    #[test]
    fn currencies_match_ok() {
        assert_eq!(
            Price::identity(),
            Price::new(
                Coin::<SuperGroupTestC2>::new(4),
                Coin::<SuperGroupTestC2>::new(4)
            )
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
