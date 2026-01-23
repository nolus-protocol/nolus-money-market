use std::ops::{Add, AddAssign};

use crate::{
    coin::{Amount, Coin},
    fraction::{Coprime, ToFraction, Unit as FractionUnit},
    price::Price,
    ratio::SimpleFraction,
};

impl<C, QuoteC> Price<C, QuoteC>
where
    C: 'static,
    QuoteC: 'static,
{
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

    /// Multiplication with а potential loss of precision
    ///
    /// In case the nominator or denominator overflows, they both are trimmed with so many bits as necessary for
    /// the larger value to fit within the price amount limits. If that would make any of them get to zero,
    /// then return [None].
    ///
    /// Price(amount, amount_quote) * SimpleFraction(nominator / denominator) = Price(amount * denominator, amount_quote * nominator)
    /// where the pairs (amount, nominator) and (amount_quote, denominator) are transformed into co-prime numbers.
    /// Please note that Price(amount, amount_quote) is like SimpleFraction(amount_quote / amount).
    pub fn lossy_mul<F>(self, rhs: F) -> Option<Self>
    where
        F: ToFraction<Amount>,
    {
        self.to_fraction()
            .lossy_mul(rhs.to_fraction())
            .map(Self::from_fraction)
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

        super::total(factored_amount, self)
            .zip(super::total(factored_amount, rhs))
            .and_then(|(factored_self, factored_rhs)| factored_self.checked_add(factored_rhs))
            .map(|factored_total| super::total_of(factored_amount).is(factored_total))
    }

    fn from_fraction(fraction: SimpleFraction<Amount>) -> Self {
        Self::new(
            Coin::new(fraction.denominator()),
            Coin::new(fraction.nominator()),
        )
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

impl<C, Q> ToFraction<Amount> for Price<C, Q> {
    fn to_fraction(self) -> SimpleFraction<Amount> {
        SimpleFraction::new(self.amount_quote.to_primitive(), self.amount.to_primitive())
    }
}

/// Represents the financial cross rate operation
pub trait CrossPrice<Rhs> {
    type Output;

    fn cross_with(self, rhs: Rhs) -> Self::Output;
}

impl<C, QuoteC, QuoteQuoteC> CrossPrice<Price<QuoteC, QuoteQuoteC>> for Price<C, QuoteC>
where
    C: 'static,
    QuoteC: 'static,
    QuoteQuoteC: 'static,
{
    type Output = Option<Price<C, QuoteQuoteC>>;

    #[track_caller]
    fn cross_with(self, rhs: Price<QuoteC, QuoteQuoteC>) -> Self::Output {
        // Price(a, b) * Price(c, d) = Price(a, d) * SimpleFraction(b / c)
        // Please note that Price(amount, amount_quote) is like SimpleFraction(amount_quote / amount).

        Price::new(self.amount, rhs.amount_quote).lossy_mul(SimpleFraction::new(
            self.amount_quote.to_primitive(),
            rhs.amount.to_primitive(),
        ))
    }
}

#[cfg(test)]
mod test {
    use std::ops::{Add, AddAssign};

    use sdk::cosmwasm_std::{Uint128, Uint256};

    use crate::{
        coin::Amount,
        price::{
            self, CrossPrice, Price,
            test::{Coin, QuoteCoin, QuoteQuoteCoin, price},
        },
        ratio::SimpleFraction,
        test::coin,
    };

    #[test]
    fn from_fraction() {
        let expect = price(coin::coin2(1), coin::coin1(4));
        assert_eq!(
            expect,
            Price::from_fraction(SimpleFraction::new(4u128, 1u128))
        );
    }

    #[test]
    fn add_no_round() {
        add_impl(
            coin::coin2(1),
            coin::coin1(2),
            coin::coin2(5),
            coin::coin1(10),
            coin::coin2(1),
            coin::coin1(4),
        );
        add_impl(
            coin::coin2(2),
            coin::coin1(1),
            coin::coin2(10),
            coin::coin1(5),
            coin::coin2(1),
            coin::coin1(1),
        );
        add_impl(
            coin::coin2(2),
            coin::coin1(3),
            coin::coin2(10),
            coin::coin1(14),
            coin::coin2(10),
            coin::coin1(29),
        );
    }

    #[test]
    fn add_round() {
        add_impl(
            coin::coin2(Amount::MAX),
            coin::coin1(1),
            coin::coin2(1),
            coin::coin1(1),
            coin::coin2(1),
            coin::coin1(1),
        );
    }

    #[test]
    fn lossy_add_no_round() {
        lossy_add_impl(
            coin::coin2(1),
            coin::coin1(2),
            coin::coin2(5),
            coin::coin1(10),
            coin::coin2(1),
            coin::coin1(4),
        );
        lossy_add_impl(
            coin::coin2(2),
            coin::coin1(1),
            coin::coin2(10),
            coin::coin1(5),
            coin::coin2(1),
            coin::coin1(1),
        );
        lossy_add_impl(
            coin::coin2(2),
            coin::coin1(3),
            coin::coin2(10),
            coin::coin1(14),
            coin::coin2(10),
            coin::coin1(29),
        );
    }

    #[test]
    fn lossy_add_round() {
        // 1/3 + 2/7 = 13/21 that is 0.(619047)*...
        let amount_exp = 1_000_000_000_000_000_000;
        let quote_exp = 619_047_619_047_619_047;
        lossy_add_impl(
            coin::coin2(3),
            coin::coin1(1),
            coin::coin2(7),
            coin::coin1(2),
            coin::coin2(amount_exp),
            coin::coin1(quote_exp),
        );
        lossy_add_impl(
            coin::coin2(amount_exp),
            coin::coin1(quote_exp),
            coin::coin2(3),
            coin::coin1(1),
            coin::coin2(amount_exp),
            coin::coin1(quote_exp + 333_333_333_333_333_333),
        );
        lossy_add_impl(
            coin::coin2(amount_exp + 1),
            coin::coin1(quote_exp),
            coin::coin2(21),
            coin::coin1(1),
            coin::coin2(amount_exp / 5),
            coin::coin1(133_333_333_333_333_333),
        );

        lossy_add_impl(
            coin::coin2(amount_exp + 1),
            coin::coin1(1),
            coin::coin2(1),
            coin::coin1(1),
            coin::coin2(1),
            coin::coin1(1),
        );

        lossy_add_impl(
            coin::coin2(Amount::MAX),
            coin::coin1(1),
            coin::coin2(1),
            coin::coin1(1),
            coin::coin2(1),
            coin::coin1(1),
        );
    }

    #[test]
    fn lossy_add_overflow() {
        // 2^128 / FACTOR (10^18) / 2^64 ~ 18.446744073709553
        let p1 = price::total_of(coin::coin2(1)).is(coin::coin1(Amount::from(u64::MAX) * 19u128));
        let p2 = Price::identity();
        assert!(p1.lossy_add(p2).is_none())
    }

    #[test]
    fn lossy_mul_no_round() {
        lossy_mul_impl(
            coin::coin2(1),
            coin::coin1(2),
            coin::coin1(2),
            qq(1),
            coin::coin2(1),
            qq(1),
        );
        lossy_mul_impl(
            coin::coin2(2),
            coin::coin1(3),
            coin::coin1(18),
            qq(5),
            coin::coin2(12),
            qq(5),
        );
        lossy_mul_impl(
            coin::coin2(7),
            coin::coin1(3),
            coin::coin1(11),
            qq(21),
            coin::coin2(11),
            qq(9),
        );
        lossy_mul_impl(
            coin::coin2(7),
            coin::coin1(3),
            coin::coin1(11),
            qq(23),
            coin::coin2(7 * 11),
            qq(3 * 23),
        );

        let big_int = Amount::MAX - 1;
        assert!(big_int % 3 != 0 && big_int % 11 != 0);
        lossy_mul_impl(
            coin::coin2(big_int),
            coin::coin1(3),
            coin::coin1(11),
            qq(big_int),
            coin::coin2(11),
            qq(3),
        );

        assert_eq!(0, Amount::MAX % 5);
        lossy_mul_impl(
            coin::coin2(Amount::MAX),
            coin::coin1(2),
            coin::coin1(3),
            qq(5),
            coin::coin2(Amount::MAX / 5 * 3),
            qq(2),
        );
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
        assert_eq!(Some(exp), price1.cross_with(price2));

        let price3 = price::total_of(amount1).is(quote2);
        let ratio = SimpleFraction::new(quote1, amount2);
        assert_eq!(Some(exp), price3.lossy_mul(ratio));
    }

    fn lossy_mul_shifts_impl(q1: Amount, shifts: u8) {
        let a1 = Amount::MAX - 1;
        let a2: Amount = 1 << shifts;
        let q2 = a2 / q1 + 3; // the aim is q1 * q2 > a2

        assert!(a1 % q1 != 0);
        assert!(a1 % q2 != 0);
        assert!(a2 % q1 != 0);
        assert!(a2 % q2 != 0);
        assert_eq!(1, a2 >> shifts);

        let a_exp = shift_product(a1, a2, shifts);
        let q_exp = shift_product(q1, q2, shifts);
        lossy_mul_impl(
            coin::coin2(a1),
            coin::coin1(q1),
            coin::coin1(a2),
            qq(q2),
            coin::coin2(a_exp),
            qq(q_exp),
        );
    }

    fn lossy_mul_overflow_impl(a1: Amount, q1: Amount, a2: Amount, q2: Amount, shifts: u8) {
        assert!(a1 % q1 != 0);
        assert!(a1 % q2 != 0);
        assert!(a2 % q1 != 0);
        assert!(a2 % q2 != 0);

        assert!(shift_product(a1, a2, shifts) == 0 || shift_product(q1, q2, shifts) == 0);
        let price1 = price::total_of(coin::coin2(a1)).is(coin::coin1(q1));
        let price2 = price::total_of(coin::coin1(a2)).is(qq(q2));
        assert_eq!(None, price1.cross_with(price2));
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

    fn qq(a: Amount) -> QuoteQuoteCoin {
        QuoteQuoteCoin::new(a)
    }
}
