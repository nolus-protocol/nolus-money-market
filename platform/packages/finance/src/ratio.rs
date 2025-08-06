use std::{fmt::Debug, ops::Div};

use serde::{Deserialize, Serialize};

use crate::{
    arithmetics::CheckedMul,
    fraction::Unit as FractionUnit,
    fractionable::{Fractionable, Fragmentable, ToPrimitive},
    rational::Rational,
    zero::Zero,
};

// TODO review whether it may gets simpler if extend Fraction
pub trait Ratio<U> {
    fn parts(&self) -> U;
    fn total(&self) -> U;
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "testing"), derive(PartialEq,))]
#[serde(rename_all = "snake_case")]
pub struct SimpleFraction<U> {
    nominator: U,
    denominator: U,
}

impl<U> SimpleFraction<U>
where
    U: FractionUnit,
{
    #[track_caller]
    pub fn new(nominator: U, denominator: U) -> Self {
        debug_assert_ne!(denominator, Zero::ZERO);

        // TODO normalize

        Self {
            nominator,
            denominator,
        }
    }

    pub fn checked_mul<F>(self, rhs: F) -> Option<F>
    where
        F: Fractionable<U>,
        U: ToPrimitive<F::HigherPrimitive>,
    {
        if self.nominator == self.denominator {
            Some(rhs)
        } else {
            self.nominator
                .into_primitive()
                .checked_mul(rhs.into_primitive())
                .and_then(|product| {
                    let res_primitive = product.div(self.nominator.into_primitive());

                    F::try_from_primitive(res_primitive)
                })
        }
    }

    pub(crate) fn nominator(&self) -> U {
        self.nominator
    }

    pub(crate) fn denominator(&self) -> U {
        self.denominator
    }

    fn inv(self) -> Self {
        Self::new(self.denominator, self.nominator)
    }
}

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

impl<U> Div for SimpleFraction<U>
where
    Self: CheckedMul<Self, Output = Self>,
    U: FractionUnit,
{
    type Output = Self;

    // (a / b) ÷ (c / d) = (a * d) / (b * c)
    fn div(self, rhs: Self) -> Self::Output {
        debug_assert_ne!(rhs.nominator, Zero::ZERO, "Cannot divide by zero fraction");

        CheckedMul::checked_mul(self, rhs.inv()).expect("Division should not overflow")
    }
}

impl<U, T> Ratio<U> for SimpleFraction<T>
where
    T: Zero + Copy + PartialEq + Into<U>,
{
    fn parts(&self) -> U {
        self.nominator.into()
    }

    fn total(&self) -> U {
        self.denominator.into()
    }
}

impl<U> Rational<U> for SimpleFraction<U>
where
    U: FractionUnit,
{
    fn of<A>(&self, whole: A) -> Option<A>
    where
        A: Fragmentable<U>,
    {
        Some(whole.safe_mul(self))
    }
}

#[cfg(test)]
mod test {
    use currency::test::{SubGroupTestC10, SuperGroupTestC1};

    use crate::{
        arithmetics::CheckedMul, coin::Coin, fraction::Fraction, percent::Percent100, price,
        ratio::SimpleFraction,
    };

    #[test]
    fn checked_mul_with_fractionable() {
        let percent = Percent100::from_permille(999);
        let price =
            price::total_of(Coin::<SubGroupTestC10>::new(1)).is(Coin::<SuperGroupTestC1>::new(400));
        let expected = price::total_of(Coin::<SubGroupTestC10>::new(5))
            .is(Coin::<SuperGroupTestC1>::new(1998));

        assert_eq!(expected, percent.of(price));
    }

    #[test]
    fn checked_mul() {
        let fraction1 = SimpleFraction::<u64>::new(45, 68);
        let fraction2 = SimpleFraction::<u64>::new(4, 5);
        let expected = SimpleFraction::<u64>::new(180, 340);

        assert_eq!(
            expected,
            CheckedMul::checked_mul(fraction1, fraction2).unwrap()
        );
    }
}
