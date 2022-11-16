use serde::{Deserialize, Serialize};

use sdk::schemars::{self, JsonSchema};

use crate::{
    coin::{Amount, Coin},
    currency::{self, Currency},
    fraction::Fraction,
    fractionable::HigherRank,
    ratio::Rational,
};

pub mod dto;

pub fn total_of<C>(amount: Coin<C>) -> PriceBuilder<C>
where
    C: Currency,
{
    debug_assert!(!amount.is_zero());
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
        debug_assert!(!to.is_zero());
        Price::new(self.0, to)
    }
}

type DoubleAmount = <Amount as HigherRank<Amount>>::Type;
type IntermediateAmount = <Amount as HigherRank<Amount>>::Intermediate;

/// Represents the price of a currency in a quote currency, ref: <https://en.wikipedia.org/wiki/Currency_pair>
///
/// The price is always kept in a canonical form of the underlying ratio. The simplifies equality and comparison operations.
/// For example, Price<EUR, USD> 1.15, generally represented as EURUSD or EUR/USD, means that one EUR is exchanged for 1.15 USD.
/// Both amounts a price is composed of should be non-zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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
    fn new(amount: Coin<C>, amount_quote: Coin<QuoteC>) -> Self {
        debug_assert!(
            Amount::from(amount) == Amount::from(amount_quote) || !currency::equal::<C, QuoteC>()
        );

        let (amount_normalized, amount_quote_normalized) = amount.into_coprime_with(amount_quote);
        Self {
            amount: amount_normalized,
            amount_quote: amount_quote_normalized,
        }
    }

    /// Returns a new [`Price`] which represents identity mapped, one to one, currency pair.
    pub fn identity() -> Self {
        Self {
            amount: Coin::new(1),
            amount_quote: Coin::new(1),
        }
    }

    /// Add two prices rounding each of them to 1.10-18, simmilarly to
    /// the precision provided by cosmwasm::Decimal.
    ///
    /// TODO Implement a variable precision algorithm depending on the
    /// value of the prices. The rounding would be done by shifting to
    /// the right both amounts of the price with a bigger denominator
    /// until a * d + b * c and b * d do not overflow.
    pub fn lossy_add(self, rhs: Self) -> Self {
        const FACTOR: Amount = 1_000_000_000_000_000_000; // 1*10^18
        let factored_amount = FACTOR.into();
        let factored_total = total(factored_amount, self) + total(factored_amount, rhs);
        total_of(factored_amount).is(factored_total)
    }

    pub fn lossy_mul<QuoteQuoteC>(self, rhs: Price<QuoteC, QuoteQuoteC>) -> Price<C, QuoteQuoteC>
    where
        QuoteQuoteC: Currency,
    {
        // Price(a, b) * Price(c, d) = Price(a * c, b * d)
        // first try to convert (a, d) and (b, c) into co-prime numbers
        let (amount_normalized, rhs_amount_quote_normalized) =
            self.amount.into_coprime_with(rhs.amount_quote);
        let (amount_quote_normalized, rhs_amount_normalized) =
            self.amount_quote.into_coprime_with(rhs.amount);

        let double_amount =
            DoubleAmount::from(amount_normalized) * DoubleAmount::from(rhs_amount_normalized);
        let double_amount_quote = DoubleAmount::from(amount_quote_normalized)
            * DoubleAmount::from(rhs_amount_quote_normalized);

        let extra_bits =
            Self::bits_above_max(double_amount).max(Self::bits_above_max(double_amount_quote));

        Price::new(
            Self::trim_down(double_amount, extra_bits).into(),
            Self::trim_down(double_amount_quote, extra_bits).into(),
        )
    }

    pub fn inv(self) -> Price<QuoteC, C> {
        Price {
            amount: self.amount_quote,
            amount_quote: self.amount,
        }
    }

    #[track_caller]
    fn bits_above_max(double_amount: DoubleAmount) -> u32 {
        const BITS_MAX_AMOUNT: u32 = Amount::BITS;
        let higher_half: Amount = IntermediateAmount::try_from(double_amount >> BITS_MAX_AMOUNT)
            .expect("Bigger Amount Higher Rank Type than required!")
            .into();
        BITS_MAX_AMOUNT - higher_half.leading_zeros()
    }

    #[track_caller]
    fn trim_down(double_amount: DoubleAmount, bits: u32) -> Amount {
        debug_assert!(bits <= Amount::BITS);
        let amount: IntermediateAmount = (double_amount >> bits)
            .try_into()
            .expect("insufficient bits to trim");
        amount.into()
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
    use sdk::cosmwasm_std::{Uint128, Uint256};

    use crate::{
        coin::{Amount, Coin as CoinT},
        currency::{Currency, SymbolStatic},
        price::{self, Price},
        test::currency::{Nls, Usdc},
    };

    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
    struct QuoteQuoteCurrency {}
    impl Currency for QuoteQuoteCurrency {
        const TICKER: SymbolStatic = "mycutecoin";
        const BANK_SYMBOL: SymbolStatic = "ibc/dcnqweuio2938fh2f";
        const DEX_SYMBOL: SymbolStatic = "ibc/cme72hr2";
    }
    type QuoteQuoteCoin = CoinT<QuoteQuoteCurrency>;
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

    #[test]
    fn add() {
        lossy_add_impl(c(1), q(2), c(5), q(10), c(1), q(4));
        lossy_add_impl(c(2), q(1), c(10), q(5), c(1), q(1));
        lossy_add_impl(c(2), q(3), c(10), q(14), c(10), q(29));
    }

    #[test]
    fn lossy_add() {
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
    #[should_panic]
    fn lossy_add_overflow() {
        // 2^128 / FACTOR (10^18) / 2^64 ~ 18.446744073709553
        let p1 = price::total_of(c(1)).is(q(u128::from(u64::MAX) * 19u128));
        let p2 = Price::identity();
        p1.lossy_add(p2);
    }

    #[test]
    fn mul() {
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
        let a1 = u128::MAX - 1;
        let q1 = 5;
        let a2: Amount = 3;
        let q2 = 7;
        assert!(a1 % q1 != 0 && a1 % q2 != 0);
        assert!(a2 % q1 != 0 && a2 % q2 != 0);
        const SHIFTS: u32 = 2;
        assert_eq!(0, a2 >> SHIFTS);
        let a_exp = shift_product(a1, a2, SHIFTS);
        let q_exp = shift_product(q1, q2, SHIFTS);
        lossy_mul_impl(c(a1), q(q1), q(a2), qq(q2), c(a_exp), qq(q_exp));
    }

    #[test]
    fn lossy_mul_many_shifts() {
        let a = u128::MAX - 1;
        let q1 = u128::MAX;
        let q2 = 7;
        assert!(a % q1 != 0 && a % q2 != 0);
        const SHIFTS: u32 = 128;
        let a_exp = shift_product(a, a, SHIFTS);
        let q_exp = shift_product(q1, q2, SHIFTS);
        lossy_mul_impl(c(a), q(q1), q(a), qq(q2), c(a_exp), qq(q_exp));
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
        assert_eq!(exp, price1.lossy_add(price2));
    }

    fn shift_product<A1, A2>(a1: A1, a2: A2, shifts: u32) -> Amount
    where
        A1: Into<Uint256>,
        A2: Into<Uint256>,
    {
        Uint128::try_from((a1.into() * a2.into()) >> shifts)
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
        assert_eq!(exp, price1.lossy_mul(price2));
    }
}
