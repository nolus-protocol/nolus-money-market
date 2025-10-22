use std::ops::{Div, Mul, Shr};

use crate::{
    coin::Amount,
    fraction::{Coprime, Unit as FractionUnit},
    fractionable::{Fractionable, IntoMax, ToDoublePrimitive, TryFromMax, checked_mul::CheckedMul},
    ratio::SimpleFraction,
    zero::Zero,
};

impl<U> SimpleFraction<U>
where
    U: FractionUnit,
{
    pub fn checked_mul<M>(&self, rhs: M) -> Option<M>
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

    /// Multiplies two `SimpleFraction`-s with possible precision lost
    pub fn lossy_mul(&self, rhs: Self) -> Self
    where
        U: Bits + Coprime + TryFromMax<<U as ToDoublePrimitive>::Double>,
        <U as ToDoublePrimitive>::Double:
            Clone + Mul<Output = <U as ToDoublePrimitive>::Double> + Shr<u32, Output = U::Double>,
    {
        let (lhs, rhs) = self.cross_normalize(rhs);
        let double_nom = lhs.nominator.to_double().mul(rhs.nominator.to_double());
        let double_denom = lhs.denominator.to_double().mul(rhs.denominator.to_double());

        let extra_bits = bits_above_max::<U, _>(double_nom.clone())
            .max(bits_above_max::<U, _>(double_denom.clone()));

        Self::new(
            trim_down::<U, _>(double_nom, extra_bits),
            trim_down::<U, _>(double_denom, extra_bits),
        )
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
fn bits_above_max<U, D>(double: D) -> u32
where
    U: Bits + TryFromMax<D>,
    D: Shr<u32, Output = D>,
{
    let bits_max: u32 = U::BITS;
    let higher_half =
        U::try_from_max(double >> bits_max).expect("Bigger Double Type than required!");
    bits_max - higher_half.leading_zeros()
}

#[track_caller]
fn trim_down<U, D>(double: D, bits: u32) -> U
where
    U: Bits + PartialOrd + TryFromMax<D> + Zero,
    D: Shr<u32, Output = D>,
{
    let trimmed_unit = U::try_from_max(double >> bits).expect("insufficient bits to trim");
    assert!(trimmed_unit > U::ZERO, "overflow during multiplication");
    trimmed_unit
}

pub trait Bits {
    const BITS: u32;

    fn leading_zeros(&self) -> u32;
}

impl Bits for Amount {
    const BITS: u32 = Self::BITS;

    fn leading_zeros(&self) -> u32 {
        Amount::leading_zeros(*self)
    }
}

/// Checked multiplication of two `SimpleFraction`-s
/// Returns `None` if either the numerator or denominator multiplication overflows
impl<U> CheckedMul for SimpleFraction<U>
where
    U: CheckedMul<U, Output = U> + FractionUnit,
{
    type Output = Self;

    fn checked_mul(self, rhs: Self) -> Option<Self::Output> {
        // (a / b).checked_mul(c / d) = (a / d).checked_mul(c / b)
        // => (a1.checked_mul(c1)) / (d1.checked_mul(b1))
        // where a1, b1, c1 and d1 are normalized
        let (ad, cb) = self.cross_normalize(rhs);

        ad.nominator
            .checked_mul(cb.nominator)
            .and_then(|nominator| {
                ad.denominator
                    .checked_mul(cb.denominator)
                    .map(|denominator| Self::new(nominator, denominator))
            })
    }
}

impl<U> Div for SimpleFraction<U>
where
    U: CheckedMul<U, Output = U> + Coprime,
{
    type Output = Self;

    // (a / b) ÷ (c / d) = (a * d) / (b * c)
    fn div(self, rhs: Self) -> Self::Output {
        debug_assert_ne!(rhs.nominator, Zero::ZERO, "Cannot divide by zero fraction");

        self.checked_mul(rhs.inv())
            .expect("Division should not overflow")
    }
}

#[cfg(test)]
mod test {
    use std::ops::Div;

    use bnum::types::U256;

    use crate::{
        coin::Amount, fractionable::checked_mul::CheckedMul, percent::Units as PercentUnits,
        ratio::SimpleFraction,
    };

    #[test]
    fn lossy_mul() {
        assert_eq!(fraction(3, 10), fraction(3, 4).lossy_mul(fraction(2, 5)));
        assert_eq!(
            fraction(Amount::MAX, 20),
            fraction(Amount::MAX, 4).lossy_mul(fraction(1, 5))
        );
        assert_eq!(
            fraction(3, 2),
            fraction(Amount::MAX, 4).lossy_mul(fraction(6, Amount::MAX))
        );
        assert_eq!(
            fraction(1, 2),
            fraction(Amount::MAX / 3, 4).lossy_mul(fraction(6, Amount::MAX - 1))
        );
    }

    #[test]
    fn lossy_mul_with_trim() {
        assert_eq!(
            fraction(Amount::MAX - 1, 27 >> 1),
            fraction(Amount::MAX - 1, 3).lossy_mul(fraction(2, 9))
        );
        assert_eq!(
            fraction(Amount::MAX - 1, 27 >> 1),
            fraction(Amount::MAX / 2, 3).lossy_mul(fraction(4, 9))
        );
    }

    #[test]
    #[should_panic = "overflow"]
    fn lossy_mul_panic() {
        let lhs = fraction(Amount::MAX / 5, 3);
        let rhs = fraction(Amount::MAX / 2, 7);

        let _ = lhs.lossy_mul(rhs);
    }

    #[test]
    fn cross_normalize() {
        let a = fraction(12, 25);
        let b = fraction(35, 9);

        assert_eq!((fraction(4, 3), fraction(7, 5)), a.cross_normalize(b))
    }

    #[test]
    fn checked_mul_trait() {
        let lhs = SimpleFraction::new(u_256(350), u_256(1000));
        let rhs = SimpleFraction::new(u_256(400), u_256(1000));
        let exp = SimpleFraction::new(u_256(7), u_256(50));
        assert_eq!(exp, lhs.checked_mul(rhs).unwrap())
    }

    #[test]
    fn checked_mul_trait_overflow() {
        let lhs = SimpleFraction::new(U256::MAX - u_256(1), u_256(1000));
        let rhs = SimpleFraction::new(u_256(3), u_256(1000));
        assert!(lhs.checked_mul(rhs).is_none())
    }

    #[test]
    fn div() {
        assert_eq!(fraction(5, 4), fraction(45, 32).div(fraction(9, 8)))
    }

    fn fraction(nom: Amount, denom: Amount) -> SimpleFraction<Amount> {
        SimpleFraction::new(nom, denom)
    }

    fn u_256(quantity: PercentUnits) -> U256 {
        U256::from(quantity)
    }
}
