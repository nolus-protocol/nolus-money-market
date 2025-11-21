use std::{
    fmt::Debug,
    ops::{Add, AddAssign, Mul},
};

use serde::{Deserialize, Serialize};

use crate::{
    coin::{Amount, Coin},
    error::{Error, Result},
    fraction::{Coprime, Unit as FractionUnit},
    fractionable::ToDoublePrimitive,
    ratio::{SimpleFraction, multiplication::Bits},
    rational::Rational,
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

    /// Multiplication with а potential loss of precision
    ///
    /// In case the nominator or denominator overflows, they both are trimmed with so many bits as necessary for
    /// the larger value to fit within the price amount limits. If that would make any of them get to zero,
    /// then return [None].
    ///
    /// Price(amount, amount_quote) * Ratio(nominator / denominator) = Price(amount * denominator, amount_quote * nominator)
    /// where the pairs (amount, nominator) and (amount_quote, denominator) are transformed into co-prime numbers.
    /// Please note that Price(amount, amount_quote) is like SimpleFraction(amount_quote / amount).
    pub fn lossy_mul<F, U>(self, rhs: F) -> Option<Self>
    where
        F: Into<SimpleFraction<U>>,
        U: Bits + FractionUnit + Into<Amount>,
    {
        self.map_with_fraction(|self_as_fraction| {
            self_as_fraction.lossy_mul(rhs.into().to_amount_fraction())
        })
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

    fn map_with_fraction<WithFraction>(self, f: WithFraction) -> Option<Self>
    where
        WithFraction: FnOnce(SimpleFraction<Amount>) -> Option<SimpleFraction<Amount>>,
    {
        f(SimpleFraction::new(
            Amount::from(self.amount_quote),
            Amount::from(self.amount),
        ))
        .map(Self::from_fraction)
    }

    fn from_fraction<U>(fraction: SimpleFraction<U>) -> Self
    where
        U: FractionUnit + Into<Amount>,
    {
        Self::new(
            Coin::new(fraction.denominator().into()),
            Coin::new(fraction.nominator().into()),
        )
    }

    pub fn checked_add(self, rhs: Self) -> Option<Self> {
        // let a1 = a / gcd(a, c), and c1 = c / gcd(a, c), then
        // b / a + d / c = (b * c1 + d * a1) / (a1 * c1 * gcd(a, c))
        // taking into account that Price is like amount_quote/amount
        let (a1, c1) = self.amount.to_coprime_with(rhs.amount);
        debug_assert_eq!(0, Amount::from(self.amount) % Amount::from(a1));
        debug_assert_eq!(0, Amount::from(rhs.amount) % Amount::from(c1));
        let gcd: Amount = match self.amount.checked_div(a1.into()) {
            None => unreachable!("invariant on amount != 0 should have passed!"),
            Some(gcd) => gcd.into(),
        };
        debug_assert_eq!(Some(Coin::new(gcd)), rhs.amount.checked_div(c1.into()));

        let may_b_c1 = self.amount_quote.checked_mul(c1.into());
        let may_d_a1 = rhs.amount_quote.checked_mul(a1.into());
        let may_amount_quote = may_b_c1
            .zip(may_d_a1)
            .and_then(|(b_c1, d_a1)| b_c1.checked_add(d_a1));
        let may_amount = a1
            .checked_mul(c1.into())
            .and_then(|a1_c1| a1_c1.checked_mul(gcd));
        may_amount_quote
            .zip(may_amount)
            .map(|(amount_quote, amount)| Self::new(amount, amount_quote))
    }

    /// Add two prices rounding each of them to 1.10-18, simmilarly to
    /// the precision provided by CosmWasm's ['Decimal'][sdk::cosmwasm_std::Decimal].
    ///
    /// TODO Implement a variable precision algorithm depending on the
    /// value of the prices. The rounding would be done by shifting to
    /// the right both amounts of the price with a bigger denominator
    /// until a * d + b * c and b * d do not overflow.
    fn lossy_add(self, rhs: Self) -> Option<Self> {
        const FACTOR: Amount = 1_000_000_000_000_000_000; // 1*10^18
        let factored_amount = Coin::new(FACTOR);

        total(factored_amount, self)
            .zip(total(factored_amount, rhs))
            .and_then(|(factored_self, factored_rhs)| factored_self.checked_add(factored_rhs))
            .map(|factored_total| total_of(factored_amount).is(factored_total))
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

        let a = self.amount_quote.to_double();
        let d = other.amount.to_double();

        let b = self.amount.to_double();
        let c = other.amount_quote.to_double();
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

impl<C, QuoteC, QuoteQuoteC> Mul<Price<QuoteC, QuoteQuoteC>> for Price<C, QuoteC>
where
    C: 'static,
    QuoteC: 'static,
    QuoteQuoteC: 'static,
{
    type Output = Option<Price<C, QuoteQuoteC>>;

    #[track_caller]
    fn mul(self, rhs: Price<QuoteC, QuoteQuoteC>) -> Self::Output {
        // Price(a, b) * Price(c, d) = Price(a, d) * Rational(b / c)
        // Please note that Price(amount, amount_quote) is like Ratio(amount_quote / amount).

        Price::new(self.amount, rhs.amount_quote).lossy_mul(SimpleFraction::new(
            self.amount_quote.to_primitive(),
            rhs.amount.to_primitive(),
        ))
    }
}

/// Calculates the amount of given coins in another currency, referred here as `quote currency`.
/// Returns `None` if an overflow occurs during the calculation.
///
/// For example, total(10 EUR, 1.01 EURUSD) = 10.1 USD
pub fn total<C, QuoteC>(of: Coin<C>, price: Price<C, QuoteC>) -> Option<Coin<QuoteC>> {
    SimpleFraction::new(of.to_primitive(), price.amount.to_primitive()).of(price.amount_quote)
}

#[cfg(test)]
mod test {
    use std::ops::{Add, AddAssign, Mul};

    use currency::test::{SubGroupTestC10, SuperGroupTestC1, SuperGroupTestC2};
    use sdk::cosmwasm_std::{Uint128, Uint256};

    use crate::{
        coin::{Amount, Coin as CoinT},
        fraction::Unit,
        price::{self, Price},
        ratio::SimpleFraction,
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
            price(c(amount), q(amount_quote)),
            price(c(amount * factor), q(amount_quote * factor))
        );
    }

    #[test]
    fn eq() {
        let amount = 13;
        let amount_quote = 15;
        assert_ne!(
            price(c(amount), q(amount_quote)),
            price(c(amount), q(amount_quote + 1))
        );
        assert_ne!(
            price(c(amount - 1), q(amount_quote)),
            price(c(amount), q(amount_quote))
        );

        assert_eq!(
            price(c(amount), q(amount_quote)),
            price(c(amount), q(amount_quote))
        );

        assert_eq!(
            Price::new(q(amount_quote), c(amount)),
            price(c(amount), q(amount_quote)).inv()
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
    fn from_fraction() {
        let expect = price(c(1), q(4));
        assert_eq!(
            expect,
            Price::from_fraction(SimpleFraction::new(4u128, 1u128))
        );
    }

    #[test]
    fn total() {
        let amount_quote = 647;
        let amount = 48;
        let price = price::total_of(c(amount)).is(q(amount_quote));
        let factor = 17;
        let coin_quote = q(amount_quote * factor);
        let coin = c(amount * factor);

        assert_eq!(coin_quote, calc_total(coin, price));
        assert_eq!(coin, super::total(coin_quote, price.inv()).unwrap());
    }

    #[test]
    fn total_rounding() {
        let amount_quote = 647;
        let amount = 48;
        let price = super::total_of(c(amount)).is(q(amount_quote));
        let coin_quote = q(633);

        // 47 * 647 / 48 -> 633.5208333333334
        let coin_in = c(47);
        assert_eq!(coin_quote, calc_total(coin_in, price));

        // 633 * 48 / 647 -> 46.9613601236476
        let coin_out = c(46);
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
        let price = price::total_of(c(1)).is(q(Amount::MAX / 2 + 1));
        assert!(super::total(c(2), price).is_none());
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
        assert!(p1.lossy_add(p2).is_none())
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
    fn lossy_mul_few_shifts() {
        lossy_mul_shifts_impl(5, 1);
        lossy_mul_shifts_impl(5, 2);
        lossy_mul_shifts_impl(5, 7);
        lossy_mul_shifts_impl(5, 16);
        lossy_mul_shifts_impl(5, 63);
    }

    #[test]
    fn lossy_mul_overflow() {
        const SHIFTS: u8 = 23;
        const Q1: Amount = 7;
        const A2: Amount = 1 << SHIFTS;
        // due to a1*a2 the q1*q2 gets to 0
        lossy_mul_overflow_impl(Amount::MAX - 1, Q1, A2, A2 / Q1 - 1, SHIFTS); // the aim is q1 * q2 < a2
        // due to q1*q2 the a1*a2 gets to 0
        lossy_mul_overflow_impl(Q1, Amount::MAX - 1, A2 / Q1 - 1, A2, SHIFTS); // the aim is a1 * a2 < q2
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
        let price1 = Price::new(c(amount), q(amount_quote));
        let price2 = Price::new(c(amount), q(amount_quote + 1));
        assert!(price1 < price2);

        let total1 = calc_total(Coin::new(amount), price1);
        assert!(total1 < calc_total(Coin::new(amount), price2));
        assert_eq!(q(amount_quote), total1);
    }

    fn total_max_impl(
        amount: Amount,
        price_amount: Amount,
        price_amount_quote: Amount,
        expected: Amount,
    ) {
        let price = price::total_of(c(price_amount)).is(q(price_amount_quote));
        let expected = q(expected);
        let input = Coin::new(amount);

        assert_eq!(expected, calc_total(input, price));
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

    fn shift_product<A1, A2>(a1: A1, a2: A2, shifts: u8) -> Amount
    where
        A1: Into<Uint256>,
        A2: Into<Uint256>,
    {
        Uint128::try_from((a1.into() * a2.into()) >> u32::from(shifts))
            .expect("Incorrect test setup")
            .into()
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
        assert_eq!(Some(exp), price1.mul(price2));

        let price3 = price::total_of(amount1).is(quote2);
        let ratio = SimpleFraction::new(quote1.to_primitive(), amount2.to_primitive());
        assert_eq!(Some(exp), price3.lossy_mul(ratio));
    }

    fn lossy_mul_shifts_impl(q1: Amount, shifts: u8) {
        let a1 = u128::MAX - 1;
        let a2: Amount = 1 << shifts;
        let q2 = a2 / q1 + 3; // the aim is q1 * q2 > a2

        assert!(a1 % q1 != 0);
        assert!(a1 % q2 != 0);
        assert!(a2 % q1 != 0);
        assert!(a2 % q2 != 0);
        assert_eq!(1, a2 >> shifts);

        let a_exp = shift_product(a1, a2, shifts);
        let q_exp = shift_product(q1, q2, shifts);
        lossy_mul_impl(c(a1), q(q1), q(a2), qq(q2), c(a_exp), qq(q_exp));
    }

    fn lossy_mul_overflow_impl(a1: Amount, q1: Amount, a2: Amount, q2: Amount, shifts: u8) {
        assert!(a1 % q1 != 0);
        assert!(a1 % q2 != 0);
        assert!(a2 % q1 != 0);
        assert!(a2 % q2 != 0);

        assert!(shift_product(a1, a2, shifts) == 0 || shift_product(q1, q2, shifts) == 0);
        let price1 = price::total_of(c(a1)).is(q(q1));
        let price2 = price::total_of(q(a2)).is(qq(q2));
        assert_eq!(None, price1.mul(price2));
    }

    fn price(amount: Coin, amount_quote: QuoteCoin) -> Price<SuperGroupTestC2, SuperGroupTestC1> {
        Price::new(amount, amount_quote)
    }

    fn calc_total(coin: Coin, price: Price<SuperGroupTestC2, SuperGroupTestC1>) -> QuoteCoin {
        super::total(coin, price).unwrap()
    }
}

#[cfg(test)]
mod test_lossy {
    use currency::test::{SubGroupTestC10, SuperGroupTestC1};

    use crate::coin::{Amount, Coin};

    mod percent {
        use crate::{
            percent::Percent100,
            price::{self},
        };

        #[test]
        fn greater_than_one() {
            let price = price::total_of(super::c(1)).is(super::q(1000));
            let percent = Percent100::from_permille(1);
            assert_eq!(
                price.lossy_mul::<_, u128>(percent),
                Some(price::total_of(super::c(1)).is(super::q(1)))
            );
        }

        #[test]
        fn less_than_one() {
            let price = price::total_of(super::c(10)).is(super::q(1));
            let twenty_percents = Percent100::from_percent(20);
            assert_eq!(
                price.lossy_mul::<_, u128>(twenty_percents),
                Some(price::total_of(super::c(50)).is(super::q(1)))
            );
        }
    }

    mod u128_ratio {
        use currency::test::{SubGroupTestC10, SuperGroupTestC1};

        use crate::{
            coin::{Amount, Coin},
            price::{self},
            ratio::SimpleFraction,
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
        #[should_panic = "overflow"]
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
                price.lossy_mul(ratio).expect("overflow"),
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
