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

    pub fn lossy_mul(&self, rhs: Self) -> Self
    where
        U: Bits + Coprime + TryFromMax<<U as ToDoublePrimitive>::Double>,
        <U as ToDoublePrimitive>::Double:
            Clone + Mul<Output = <U as ToDoublePrimitive>::Double> + Shr<u32, Output = U::Double>,
    {
        let (lhs_nom_norm, rhs_denom_norm) = self.nominator.to_coprime_with(rhs.denominator);
        let (lhs_denom_norm, rhs_nom_norm) = self.denominator.to_coprime_with(rhs.nominator);

        let double_nom = lhs_nom_norm.to_double().mul(rhs_nom_norm.to_double());
        let double_denom = lhs_denom_norm.to_double().mul(rhs_denom_norm.to_double());

        let extra_bits = bits_above_max::<U, _>(double_nom.clone())
            .max(bits_above_max::<U, _>(double_denom.clone()));

        Self::new(
            trim_down::<U, _>(double_nom, extra_bits),
            trim_down::<U, _>(double_denom, extra_bits),
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
    debug_assert!(bits <= U::BITS);
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
        todo!()
    }
}

// TODO unify the multiplication using the logic from SimpleFraction::checked_mul(Fractionable)
impl<U> CheckedMul for SimpleFraction<U>
where
    U: CheckedMul<U, Output = U> + FractionUnit,
{
    type Output = Self;

    fn checked_mul(self, rhs: Self) -> Option<Self::Output> {
        self.nominator
            .checked_mul(rhs.nominator)
            .and_then(|nominator| {
                self.denominator
                    .checked_mul(rhs.denominator)
                    .map(|denominator| Self::new(nominator, denominator))
            })
    }
}

#[cfg(test)]
mod test {
    use bnum::types::U256;

    use crate::{
        fractionable::checked_mul::CheckedMul, percent::Units as PercentUnits,
        ratio::SimpleFraction,
    };

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

    fn u_256(quantity: PercentUnits) -> U256 {
        U256::from(quantity)
    }
}
