use std::ops::{Add, AddAssign, Mul, Shr};

use crate::{
    coin::{Amount, Coin, DoubleCoinPrimitive},
    fraction::{Coprime, Unit as FractionUnit},
    fractionable::{ToDoublePrimitive, TryFromMax},
    percent::Units as PercentUnits,
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
    pub fn lossy_mul<F, U>(self, rhs: F) -> Option<Self>
    where
        F: Into<SimpleFraction<U>>,
        U: Bits + FractionUnit + Into<Amount>,
    {
        self.map_with_fraction(|self_as_fraction| {
            lossy_mul_inner(self_as_fraction, to_amount_fraction(rhs.into()))
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
        const FACTOR: Amount = 1_000_000_000_000_000_000; // 1*10^18
        let factored_amount = Coin::new(FACTOR);

        super::total(factored_amount, self)
            .zip(super::total(factored_amount, rhs))
            .and_then(|(factored_self, factored_rhs)| factored_self.checked_add(factored_rhs))
            .map(|factored_total| super::total_of(factored_amount).is(factored_total))
    }

    fn map_with_fraction<WithFraction>(self, f: WithFraction) -> Option<Self>
    where
        WithFraction: FnOnce(SimpleFraction<Amount>) -> Option<SimpleFraction<Amount>>,
    {
        f(SimpleFraction::new(
            self.amount_quote.to_primitive(),
            self.amount.to_primitive(),
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
}

fn lossy_mul_inner<U>(lhs: SimpleFraction<U>, rhs: SimpleFraction<U>) -> Option<SimpleFraction<U>>
where
    U: Bits + Coprime + TryFromMax<<U as ToDoublePrimitive>::Double>,
    <U as ToDoublePrimitive>::Double:
        Bits + Copy + Mul<Output = <U as ToDoublePrimitive>::Double> + Shr<u32, Output = U::Double>,
{
    let (lhs, rhs) = cross_normalize(lhs, rhs);
    let double_nom = lhs.nominator().to_double().mul(rhs.nominator().to_double());
    let double_denom = lhs
        .denominator()
        .to_double()
        .mul(rhs.denominator().to_double());

    let extra_bits = bits_above_max::<_, U>(double_nom).max(bits_above_max::<_, U>(double_denom));

    let min_precision_loss_overflow = bits(double_nom).min(bits(double_denom));

    trim_down(double_nom, extra_bits, min_precision_loss_overflow).and_then(|amount| {
        trim_down(double_denom, extra_bits, min_precision_loss_overflow)
            .map(|amount_quote| SimpleFraction::new(amount, amount_quote))
    })
}

fn to_amount_fraction<U>(fraction: SimpleFraction<U>) -> SimpleFraction<Amount>
where
    U: FractionUnit + Into<Amount>,
{
    SimpleFraction::new(fraction.nominator().into(), fraction.denominator().into())
}

fn cross_normalize<U>(
    lhs: SimpleFraction<U>,
    rhs: SimpleFraction<U>,
) -> (SimpleFraction<U>, SimpleFraction<U>)
where
    U: FractionUnit,
{
    // from (a / b) and (c / d), to (a / d) and (c / b)
    (
        SimpleFraction::new(lhs.nominator(), rhs.denominator()),
        SimpleFraction::new(rhs.nominator(), lhs.denominator()),
    )
}

#[track_caller]
fn bits<D>(double: D) -> u32
where
    D: Bits,
{
    D::BITS - double.leading_zeros()
}

#[track_caller]
fn bits_above_max<D, U>(double: D) -> u32
where
    U: Bits + FractionUnit + TryFromMax<D>,
    D: Bits,
{
    bits(double).saturating_sub(U::BITS)
}

#[track_caller]
fn trim_down<D, U>(double: D, bits_to_trim: u32, min_precision_loss_overflow: u32) -> Option<U>
where
    U: Bits + FractionUnit + TryFromMax<D>,
    D: Bits + Copy + Shr<u32, Output = D>,
{
    debug_assert!(bits_to_trim <= U::BITS);

    (bits_to_trim < min_precision_loss_overflow).then(|| trim_down_checked(double, bits_to_trim))
}

#[track_caller]
fn trim_down_checked<D, U>(double: D, bits_to_trim: u32) -> U
where
    U: Bits + FractionUnit + TryFromMax<D>,
    D: Bits + Copy + Shr<u32, Output = D>,
{
    const INSUFFICIENT_BITS: &str = "insufficient trimming bits";

    debug_assert!(
        bits_above_max::<D, U>(double) <= bits_to_trim,
        "{}",
        INSUFFICIENT_BITS
    );
    debug_assert!(
        bits_to_trim < bits(double),
        "the precision loss {bits_to_trim} exceeds the value bits {loss}",
        loss = bits(double)
    );
    let unit_amount = U::try_from_max(double >> bits_to_trim).expect(INSUFFICIENT_BITS);
    debug_assert!(
        unit_amount > U::ZERO,
        "the precision loss exceeds the value bits"
    );
    unit_amount
}

pub trait Bits {
    const BITS: u32;

    fn leading_zeros(self) -> u32;
}

impl Bits for PercentUnits {
    const BITS: u32 = Self::BITS;

    fn leading_zeros(self) -> u32 {
        self.leading_zeros()
    }
}

impl Bits for Amount {
    const BITS: u32 = Self::BITS;

    fn leading_zeros(self) -> u32 {
        self.leading_zeros()
    }
}

impl<C> Bits for Coin<C>
where
    C: 'static,
{
    const BITS: u32 = Self::BITS;

    fn leading_zeros(self) -> u32 {
        self.to_primitive().leading_zeros()
    }
}

impl Bits for DoubleCoinPrimitive {
    const BITS: u32 = Self::BITS;

    fn leading_zeros(self) -> u32 {
        self.leading_zeros()
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

#[cfg(test)]
mod test {
    use std::ops::{Add, AddAssign, Mul};

    use sdk::cosmwasm_std::{Uint128, Uint256};

    use crate::{
        coin::Amount,
        price::{
            self, Price,
            arithmetics::lossy_mul_inner,
            test::{Coin, QuoteCoin, QuoteQuoteCoin, c, price, q},
        },
        ratio::SimpleFraction,
    };

    #[test]
    fn from_fraction() {
        let expect = price(c(1), q(4));
        assert_eq!(
            expect,
            Price::from_fraction(SimpleFraction::new(4u128, 1u128))
        );
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
        let p1 = price::total_of(c(1)).is(q(Amount::from(u64::MAX) * 19u128));
        let p2 = Price::identity();
        assert!(p1.lossy_add(p2).is_none())
    }

    #[test]
    fn lossy_mul_no_round() {
        lossy_mul_impl(c(1), q(2), q(2), qq(1), c(1), qq(1));
        lossy_mul_impl(c(2), q(3), q(18), qq(5), c(12), qq(5));
        lossy_mul_impl(c(7), q(3), q(11), qq(21), c(11), qq(9));
        lossy_mul_impl(c(7), q(3), q(11), qq(23), c(7 * 11), qq(3 * 23));

        let big_int = Amount::MAX - 1;
        assert!(big_int % 3 != 0 && big_int % 11 != 0);
        lossy_mul_impl(c(big_int), q(3), q(11), qq(big_int), c(11), qq(3));

        assert_eq!(0, Amount::MAX % 5);
        lossy_mul_impl(
            c(Amount::MAX),
            q(2),
            q(3),
            qq(5),
            c(Amount::MAX / 5 * 3),
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

    #[test]
    fn lossy_mul() {
        assert_eq!(
            Some(fraction(3, 10)),
            lossy_mul_inner(fraction(3, 4), fraction(2, 5))
        );
        assert_eq!(
            Some(fraction(Amount::MAX, 20)),
            lossy_mul_inner(fraction(Amount::MAX, 4), fraction(1, 5))
        );
        assert_eq!(
            Some(fraction(3, 2)),
            lossy_mul_inner(fraction(Amount::MAX, 4), fraction(6, Amount::MAX))
        );
        assert_eq!(
            Some(fraction(1, 2)),
            lossy_mul_inner(fraction(Amount::MAX / 3, 4), fraction(6, Amount::MAX - 1))
        );
    }

    #[test]
    fn lossy_mul_inner_with_trim() {
        assert_eq!(
            Some(fraction(Amount::MAX - 1, 27 >> 1)),
            lossy_mul_inner(fraction(Amount::MAX - 1, 3), fraction(2, 9))
        );
        assert_eq!(
            Some(fraction(Amount::MAX - 1, 27 >> 1)),
            lossy_mul_inner(fraction(Amount::MAX / 2, 3), fraction(4, 9))
        );
    }

    #[test]
    fn lossy_mul_inner_panic() {
        let lhs = fraction(Amount::MAX / 5, 3);
        let rhs = fraction(Amount::MAX / 2, 7);
        assert!(lossy_mul_inner(lhs, rhs).is_none())
    }

    #[test]
    fn cross_normalize() {
        let a = fraction(12, 25);
        let b = fraction(35, 9);

        assert_eq!(
            (fraction(4, 3), fraction(7, 5)),
            super::cross_normalize(a, b)
        )
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

    fn fraction(nom: Amount, denom: Amount) -> SimpleFraction<Amount> {
        SimpleFraction::new(nom, denom)
    }
}
