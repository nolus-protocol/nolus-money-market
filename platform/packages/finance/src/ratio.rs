use std::{fmt::Debug, ops::Div};

use serde::{Deserialize, Serialize};

use crate::{
    arithmetic::{self, Bits, CheckedMul, FractionUnit, One, Trim},
    fraction::Fraction,
    fractionable::Fractionable,
    zero::Zero,
};

// TODO review whether it may gets simpler if extend Fraction
pub trait Ratio<U> {
    fn parts(&self) -> U;
    fn total(&self) -> U;
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Rational<U> {
    nominator: U,
    denominator: U,
}

impl<U> Rational<U>
where
    U: FractionUnit,
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

    /// Performs a multiplication with possibility of precision lost.
    pub fn lossy_mul<F>(self, rhs: F) -> Option<F>
    where
        F: Fractionable<U>,
    {
        if self.nominator == self.denominator {
            Some(rhs)
        } else {
            F::MaxRank::try_from(rhs).ok().and_then(|rhs_max| {
                let precise_res =
                    Self::multiply(self.nominator.into(), self.denominator.into(), rhs_max);

                precise_res.or_else(|| self.try_precise_mul(rhs))
            })
        }
    }

    fn try_precise_mul<F>(self, rhs: F) -> Option<F>
    where
        F: Fractionable<U>,
    {
        let nom = F::MaxRank::from(self.nominator);
        let denom = F::MaxRank::from(self.denominator);
        F::MaxRank::try_from(rhs).ok().and_then(|fractionable| {
            Self::precise_mul(nom, denom, fractionable, F::MaxRank::ONE)
                .map(|(max_rank_nom, max_rank_denom)| max_rank_nom.div(max_rank_denom))
                .and_then(|res| res.try_into().ok())
        })
    }

    fn precise_mul<T>(lhs_nom: T, lhs_denom: T, rhs_nom: T, rhs_denom: T) -> Option<(T, T)>
    where
        T: Bits + CheckedMul<T, Output = T> + Copy + Trim,
    {
        lhs_nom
            .checked_mul(rhs_nom)
            .and_then(|nom| lhs_denom.checked_mul(rhs_denom).map(|denom| (nom, denom)))
            .or_else(|| {
                let (lhs_nom_bits, rhs_nom_bits, extra_nom_bits) = Self::bits(lhs_nom, rhs_nom);
                let (lhs_denom_bits, rhs_denom_bits, extra_denom_bits) =
                    Self::bits(lhs_denom, rhs_denom);

                let extra_bits = extra_nom_bits.max(extra_denom_bits);

                Self::try_trim_and_mul(lhs_nom, lhs_nom_bits, rhs_nom, rhs_nom_bits, extra_bits)
                    .and_then(|trimmed_nom| {
                        Self::try_trim_and_mul(
                            lhs_denom,
                            lhs_denom_bits,
                            rhs_denom,
                            rhs_denom_bits,
                            extra_bits,
                        )
                        .map(|trimmed_denom| (trimmed_nom, trimmed_denom))
                    })
            })
    }

    fn try_trim_and_mul<T>(
        lhs: T,
        lhs_bits: u32,
        rhs: T,
        rhs_bits: u32,
        extra_bits: u32,
    ) -> Option<T>
    where
        T: CheckedMul<T, Output = T> + Trim,
    {
        let lhs_share = Self::calc_share(lhs_bits, lhs_bits + rhs_bits, extra_bits);
        let rhs_share = extra_bits - lhs_share;

        if lhs_bits <= lhs_share || rhs_bits <= rhs_share {
            None
        } else {
            let (trimmed_lhs, trimmed_rhs) = (lhs.trim(lhs_share), rhs.trim(rhs_share));
            trimmed_lhs.checked_mul(trimmed_rhs)
        }
    }

    fn calc_share(value_bits: u32, total_bits: u32, extra_bits: u32) -> u32 {
        let prod = extra_bits * value_bits;

        if 2 * (prod % total_bits) < total_bits {
            prod / total_bits
        } else {
            prod / total_bits + 1
        }
    }

    fn bits<M>(lhs: M, rhs: M) -> (u32, u32, u32)
    where
        M: Bits,
    {
        let lhs_bits = Self::significant_bits(lhs);
        let rhs_bits = Self::significant_bits(rhs);
        let total_bits = lhs_bits + rhs_bits;
        (lhs_bits, rhs_bits, total_bits.saturating_sub(M::BITS))
    }

    fn multiply<F>(parts: F::MaxRank, total: F::MaxRank, rhs: F::MaxRank) -> Option<F>
    where
        F: Fractionable<U>,
    {
        parts
            .checked_mul(rhs)
            .map(|nominator| nominator / total)
            .and_then(|res| res.try_into().ok())
    }

    #[track_caller]
    fn significant_bits<B>(value: B) -> u32
    where
        B: Bits,
    {
        let bits_max: u32 = B::BITS;
        bits_max - value.leading_zeros()
    }
}

impl<U> Bits for Rational<U>
where
    U: Bits,
{
    const BITS: u32 = U::BITS;

    fn leading_zeros(self) -> u32 {
        self.nominator
            .leading_zeros()
            .min(self.denominator.leading_zeros())
    }
}

impl<U> CheckedMul for Rational<U>
where
    U: CheckedMul<U, Output = U> + FractionUnit,
{
    type Output = Self;

    fn checked_mul(self, rhs: Self) -> Option<Self::Output> {
        todo!("Implement")
    }
}

impl<U> Div for Rational<U>
where
    U: CheckedMul<U, Output = U> + FractionUnit,
{
    type Output = Self;

    // (a / b) ÷ (c / d) = (a * d) / (b * c)
    fn div(self, rhs: Self) -> Self::Output {
        todo!("Implement")
    }
}

impl<U, T> Fraction<U> for Rational<T>
where
    Self: Ratio<U>,
{
    #[track_caller]
    fn of<A>(&self, whole: A) -> A
    where
        A: Fractionable<U>,
    {
        todo!("To remove")
    }
}

impl<U> One for Rational<U>
where
    U: FractionUnit + One,
{
    const ONE: Self = Self {
        nominator: U::ONE,
        denominator: U::ONE,
    };
}

impl<U, T> Ratio<U> for Rational<T>
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

impl<U> Trim for Rational<U>
where
    U: FractionUnit,
{
    fn trim(self, bits: u32) -> Self {
        Self::new(self.nominator.trim(bits), self.denominator.trim(bits))
    }
}
