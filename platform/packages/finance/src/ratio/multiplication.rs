use std::ops::{Div, Mul, Shr};

use crate::{
    fraction::{Coprime, Unit as FractionUnit},
    fractionable::{Fractionable, IntoMax, ToDoublePrimitive, TryFromMax, checked_mul::CheckedMul},
    ratio::{SimpleFraction, bits::Bits},
};

impl<U> SimpleFraction<U>
where
    U: FractionUnit,
{
    pub(super) fn checked_mul<M>(&self, rhs: M) -> Option<M>
    where
        U: IntoMax<M::CommonDouble>,
        M: Fractionable<U>,
    {
        if self.nominator == self.denominator {
            Some(rhs)
        } else {
            let nominator_max = self.nominator.into_max();
            let rhs_max = rhs.into_max();
            let denominator_max = self.denominator.into_max();

            nominator_max
                .checked_mul(rhs_max)
                .map(|product| product.div(denominator_max))
                .and_then(TryFromMax::try_from_max)
        }
    }

    /// Multiplication with а potential loss of precision
    ///
    /// In case the numerator or denominator overflows, they both are trimmed with so many bits as necessary for
    /// the larger value to fit within the price amount limits.
    ///
    /// SimpleFraction(numerator'′, denominator') * SimpleFraction(numerator" / denominator") = SimpleFraction(numerator' * denominator", denominator' * numerator")
    /// where the pairs (numerator', numerator") and (denominator', denominator") are transformed into co-prime numbers.
    ///
    /// Returns [None] when:
    /// * an overflow occurs regardless of bit trimming
    /// * an agressive bit trimming is required, resulting in a greater precision loss than acceptable
    pub(crate) fn lossy_mul(&self, rhs: Self) -> Option<Self>
    where
        U: Bits + Coprime + TryFromMax<<U as ToDoublePrimitive>::Double>,
        <U as ToDoublePrimitive>::Double: Bits
            + Copy
            + Mul<Output = <U as ToDoublePrimitive>::Double>
            + Shr<u32, Output = U::Double>,
    {
        let (lhs, rhs) = self.cross_normalize(rhs);
        let double_nom = lhs.nominator.to_double().mul(rhs.nominator.to_double());
        let double_denom = lhs.denominator.to_double().mul(rhs.denominator.to_double());

        let extra_bits =
            bits_above_max::<_, U>(double_nom).max(bits_above_max::<_, U>(double_denom));

        let min_precision_loss_overflow = bits(double_nom).min(bits(double_denom));

        trim_down(double_nom, extra_bits, min_precision_loss_overflow).and_then(|amount| {
            trim_down(double_denom, extra_bits, min_precision_loss_overflow)
                .map(|amount_quote| Self::new(amount, amount_quote))
        })
    }

    fn cross_normalize(&self, rhs: Self) -> (Self, Self) {
        // from (a / b) and (c / d), to (a / d) and (c / b)
        (
            Self::new(self.nominator, rhs.denominator),
            Self::new(rhs.nominator, self.denominator),
        )
    }
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

#[cfg(test)]
mod test {
    use crate::{coin::Amount, ratio::SimpleFraction};

    #[test]
    fn lossy_mul() {
        assert_eq!(
            Some(fraction(3, 10)),
            fraction(3, 4).lossy_mul(fraction(2, 5))
        );
        assert_eq!(
            Some(fraction(Amount::MAX, 20)),
            fraction(Amount::MAX, 4).lossy_mul(fraction(1, 5))
        );
        assert_eq!(
            Some(fraction(3, 2)),
            fraction(Amount::MAX, 4).lossy_mul(fraction(6, Amount::MAX))
        );
        assert_eq!(
            Some(fraction(1, 2)),
            fraction(Amount::MAX / 3, 4).lossy_mul(fraction(6, Amount::MAX - 1))
        );
    }

    #[test]
    fn lossy_mul_with_trim() {
        assert_eq!(
            Some(fraction(Amount::MAX - 1, 27 >> 1)),
            fraction(Amount::MAX - 1, 3).lossy_mul(fraction(2, 9))
        );
        assert_eq!(
            Some(fraction(Amount::MAX - 1, 27 >> 1)),
            fraction(Amount::MAX / 2, 3).lossy_mul(fraction(4, 9))
        );
    }

    #[test]
    fn lossy_mul_panic() {
        let lhs = fraction(Amount::MAX / 5, 3);
        let rhs = fraction(Amount::MAX / 2, 7);

        assert!(lhs.lossy_mul(rhs).is_none())
    }

    #[test]
    fn cross_normalize() {
        let a = fraction(12, 25);
        let b = fraction(35, 9);

        assert_eq!((fraction(4, 3), fraction(7, 5)), a.cross_normalize(b))
    }

    fn fraction(nom: Amount, denom: Amount) -> SimpleFraction<Amount> {
        SimpleFraction::new(nom, denom)
    }
}
