use std::{
    cmp::Ordering,
    fmt::Debug,
    ops::{Div, Mul, Rem},
};

use gcd::Gcd;

use serde::{Deserialize, Serialize};

use crate::{fraction::Fraction, fractionable::Fractionable, zero::Zero};

#[cfg_attr(any(test, feature = "testing"), derive(Debug))]
pub struct Ratio<U> {
    parts: U,
    total: U,
}

impl<U> Ratio<U>
where
    U: Copy + PartialEq + PartialOrd<U>,
{
    pub fn new(parts: U, total: U) -> Self {
        debug_assert!(parts <= total);

        Self { parts, total }
    }

    pub(crate) fn parts(&self) -> U {
        self.parts
    }

    pub(crate) fn total(&self) -> U {
        self.total
    }
}

impl<U> Fraction<U> for Ratio<U> {
    fn of<A>(&self, whole: A) -> A
    where
        A: Fractionable<U>,
    {
        whole.safe_mul(self)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Serialize)]
pub struct Rational<U> {
    nominator: U,
    denominator: U,
}

impl<U> Rational<U>
where
    U: Copy + Debug + Ord + PartialEq<U> + Zero,
{
    #[track_caller]
    pub fn new(nominator: U, denominator: U) -> Self {
        debug_assert_ne!(denominator, Zero::ZERO);

        Self {
            nominator,
            denominator,
        }
    }

    pub fn nominator(&self) -> U {
        self.nominator
    }

    pub fn denominator(&self) -> U {
        self.denominator
    }

    pub fn map<F, T>(&self, f: F) -> Rational<T>
    where
        F: Fn(U) -> T,
        T: Copy + Debug + Ord + Zero,
    {
        Rational::new(f(self.nominator), f(self.denominator))
    }

    pub fn to_ratio(&self) -> Option<Ratio<U>> {
        (self.nominator <= self.denominator).then(|| Ratio::new(self.nominator, self.denominator))
    }
}

pub(crate) fn into_coprime<U>(a: U, b: U) -> (U, U)
where
    U: Copy + Debug + Div<Output = U> + Gcd + Mul<Output = U> + Ord + Rem + Zero,
    <U as Rem>::Output: Debug + PartialEq + Zero,
{
    debug_assert_ne!(a, Zero::ZERO, "LHS-value is zero!");
    debug_assert_ne!(b, Zero::ZERO, "RHS-value is zero!");

    let gcd = a.gcd(b);

    debug_assert_ne!(gcd, Zero::ZERO);
    debug_assert!(
        a % gcd == Zero::ZERO,
        "LHS-value is not divisible by the GCD!"
    );
    debug_assert!(
        b % gcd == Zero::ZERO,
        "RHS-value is not divisible by the GCD!"
    );

    (a / gcd, b / gcd)
}

impl<U: PartialEq> PartialEq for Rational<U> {
    fn eq(&self, other: &Self) -> bool {
        self.nominator == other.nominator && self.denominator == other.denominator
    }
}

impl<U: PartialOrd> PartialOrd for Rational<U>
where
    U: Copy + Mul<Output = U> + Ord,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<U> Ord for Rational<U>
where
    U: Copy + Mul<Output = U> + Ord,
{
    fn cmp(&self, other: &Self) -> Ordering {
        // a/b < c/d if and only if a * d < b * c

        let a = self.nominator;
        let d = other.denominator;
        let b = self.denominator;
        let c = other.nominator;
        (a * d).cmp(&(b * c))
    }
}
pub trait CheckedMul<Rhs = Self> {
    type Output;

    fn checked_mul(self, rhs: Rhs) -> Option<Self::Output>;
}

pub trait CheckedAdd<Rhs = Self> {
    type Output;

    fn checked_add(self, rhs: Rhs) -> Option<Self::Output>;
}

pub trait CheckedDiv<Rhs = Self> {
    type Output;

    fn checked_div(self, rhs: Rhs) -> Option<Self::Output>;
}

impl<U> Rational<U>
where
    U: PartialEq<U> + Copy + PartialOrd + Div + Rem<Output = U>,
{
    pub fn checked_mul<F>(self, rhs: F) -> Option<F>
    where
        <U as Div>::Output: CheckedMul<F, Output = F>,
        F: CheckedAdd<Output = F> + Copy + Fractionable<U>,
    {
        // Rational(a,b).checked_mul(c) = (a / b).checked_mul(c) + c.safe_mul(Rational(a % b, b))

        self.nominator
            .div(self.denominator)
            .checked_mul(rhs)
            .and_then(|whole_part: F| {
                let fraction_part = rhs.safe_mul(&Ratio::new(
                    self.nominator % self.denominator,
                    self.denominator,
                ));
                whole_part.checked_add(fraction_part)
            })
    }
}

impl<U> CheckedAdd for Rational<U>
where
    U: CheckedMul<Output = U>
        + CheckedAdd<Output = U>
        + CheckedDiv<Output = U>
        + Copy
        + Debug
        + Div<Output = U>
        + Gcd
        + Mul<Output = U>
        + Ord
        + PartialEq<U>
        + Rem<Output = U>
        + Zero,
{
    type Output = Self;

    fn checked_add(self, rhs: Self) -> Option<Self::Output> {
        // let a1 = a / gcd(a, c), and c1 = c / gcd(a, c), then
        // b / a + d / c = (b * c1 + d * a1) / (a1 * c1 * gcd(a, c))
        let (a1, c1) = self::into_coprime(self.denominator, rhs.denominator);
        debug_assert_eq!(self.denominator % a1, Zero::ZERO);
        debug_assert_eq!(rhs.denominator % c1, Zero::ZERO);
        let gcd = match self.denominator.checked_div(a1) {
            None => unreachable!("invariant on amount != 0 should have passed!"),
            Some(gcd) => gcd,
        };
        debug_assert_eq!(Some(gcd), rhs.denominator.checked_div(c1));

        let may_b_c1 = self.nominator.checked_mul(c1);
        let may_d_a1 = rhs.nominator.checked_mul(a1);

        let may_nominator = may_b_c1
            .zip(may_d_a1)
            .and_then(|(b_c1, d_a1)| b_c1.checked_add(d_a1));
        let may_denominator = a1.checked_mul(c1).and_then(|a1_c1| a1_c1.checked_mul(gcd));
        may_nominator
            .zip(may_denominator)
            .map(|(nominator, denominator)| Self::new(nominator, denominator))
    }
}
