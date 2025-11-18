use std::{fmt::Debug, ops::Div};

use serde::{Deserialize, Serialize};

use crate::{
    error::{Error, Result as FinanceResult},
    fraction::{Coprime, Fraction, FractionLegacy, Unit as FractionUnit},
    fractionable::{
        Fractionable, FractionableLegacy, IntoMax, TryFromMax, checked_mul::CheckedMul,
    },
    rational::{Rational, RationalLegacy},
    zero::Zero,
};

/// A part of something that is divisible.
/// The total should be non-zero.
#[derive(Clone, Copy, Deserialize, Serialize)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug, PartialEq))]
#[serde(
    try_from = "SimpleFraction<U>",
    into = "SimpleFraction<U>",
    bound(
        serialize = "U: Clone + Serialize",
        deserialize = "U: Deserialize<'de> + FractionUnit"
    )
)]
pub struct Ratio<U>(SimpleFraction<U>);

impl<U> Ratio<U>
where
    U: FractionUnit,
{
    pub fn new(parts: U, total: U) -> Self {
        let obj = Self(SimpleFraction::new(parts, total));
        debug_assert_eq!(Ok(()), obj.invariant_held());
        obj
    }

    fn invariant_held(&self) -> FinanceResult<()> {
        Error::broken_invariant_if::<Self>(
            self.0.nominator > self.0.denominator,
            "Parts must not exceed total",
        )
    }
}

impl<U> TryFrom<SimpleFraction<U>> for Ratio<U>
where
    U: FractionUnit,
{
    type Error = Error;

    fn try_from(rational: SimpleFraction<U>) -> Result<Self, Self::Error> {
        let res = Self::new(rational.nominator, rational.denominator);
        res.invariant_held().map(|()| res)
    }
}

impl<U> From<Ratio<U>> for SimpleFraction<U>
where
    U: Clone,
{
    fn from(ratio: Ratio<U>) -> SimpleFraction<U> {
        ratio.0
    }
}

impl<U> Fraction<U> for Ratio<U>
where
    U: FractionUnit,
{
    fn of<A>(&self, whole: A) -> A
    where
        U: IntoMax<A::CommonDouble>,
        A: Fractionable<U>,
    {
        // TODO remove the full syntax when removing the RationalLegacy
        Rational::of(&self.0, whole)
            .expect("Ratio is a part of a whole, multiplication cannot overflow")
    }
}

// TODO remove when removing FractionLegacy<Units> for Percent100
impl<U> FractionLegacy<U> for Ratio<U>
where
    U: FractionUnit,
{
    fn of<A>(&self, whole: A) -> A
    where
        A: FractionableLegacy<U>,
    {
        RationalLegacy::of(&self.0, whole)
            .expect("Ratio is a part of a whole, multiplication cannot overflow")
    }
}

pub trait RatioLegacy<U> {
    fn parts(&self) -> U;
    fn total(&self) -> U;
}

/// A fraction where [denominator](Self::denominator) should be non zero
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[cfg_attr(any(test, feature = "testing"), derive(PartialEq))]
#[serde(rename_all = "snake_case")]
pub struct SimpleFraction<U> {
    nominator: U,
    denominator: U,
}

impl<U> SimpleFraction<U>
where
    U: Coprime,
{
    #[track_caller]
    pub fn new(nominator: U, denominator: U) -> Self {
        debug_assert_ne!(denominator, Zero::ZERO);

        let (nominator, denominator) = nominator.to_coprime_with(denominator);

        Self {
            nominator,
            denominator,
        }
    }

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
}

impl<U, T> RatioLegacy<U> for SimpleFraction<T>
where
    T: Copy + Into<U> + PartialEq + Zero,
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
        U: IntoMax<A::CommonDouble>,
        A: Fractionable<U>,
    {
        self.checked_mul(whole)
    }
}

// TODO remove when removing FractionLegacy<Units> for Percent100
impl<U, T> RationalLegacy<U> for SimpleFraction<T>
where
    Self: RatioLegacy<U>,
{
    fn of<A>(&self, whole: A) -> Option<A>
    where
        A: FractionableLegacy<U>,
    {
        Some(whole.safe_mul(self))
    }
}

#[cfg(test)]
mod test_ratio {

    use currency::test::SuperGroupTestC1;
    use sdk::cosmwasm_std;

    use crate::{
        coin::{Amount, Coin},
        ratio::{Ratio, SimpleFraction},
    };

    #[test]
    fn valid_invariant() {
        assert!(Ratio::new(1u32, 2u32).invariant_held().is_ok());
        assert!(Ratio::new(coin(9), coin(10)).invariant_held().is_ok());
        assert!(Ratio::new(coin(10), coin(10)).invariant_held().is_ok());
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic = "Parts must not exceed total"]
    fn invalid_variant() {
        let _ = Ratio::new(4u32, 3u32);
        let _ = Ratio::new(coin(10), coin(9));
    }

    #[test]
    #[cfg(not(debug_assertions))]
    fn invalid_variant_release() {
        let ratio = Ratio::new(4u32, 3u32);
        assert!(ratio.invariant_held().is_err());
    }

    #[test]
    fn serialize() {
        assert_eq!(
            r#"{"nominator":5,"denominator":7}"#,
            cosmwasm_std::to_json_string(&Ratio::new(5u32, 7u32)).unwrap()
        );
        assert_eq!(
            r#"{"nominator":{"amount":"5"},"denominator":{"amount":"7"}}"#,
            cosmwasm_std::to_json_string(&Ratio::new(coin(5), coin(7))).unwrap()
        );
    }

    #[test]
    fn deserialize() {
        assert_eq!(
            Ratio::new(coin(5), coin(5)),
            cosmwasm_std::from_json(r#"{"nominator":{"amount":"5"},"denominator":{"amount":"5"}}"#)
                .unwrap()
        );
    }

    #[test]
    fn serialize_deserialize_test() {
        let ratio_1 = Ratio::new(coin(3), coin(4));
        let serialized = cosmwasm_std::to_json_vec(&ratio_1).unwrap();
        assert_eq!(ratio_1, cosmwasm_std::from_json(&serialized).unwrap());

        let ratio_2 = Ratio::new(coin(Amount::MAX), coin(Amount::MAX));
        let serialized = cosmwasm_std::to_json_vec(&ratio_2).unwrap();
        assert_eq!(ratio_2, cosmwasm_std::from_json(&serialized).unwrap());
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic = "Parts must not exceed total"]
    fn serialize_deserialize_invalid_debug() {
        let _ = invalid_ratio_deserialization();
    }

    #[test]
    #[cfg(not(debug_assertions))]
    fn serialize_deserialize_invalid_release() {
        let result = invalid_ratio_deserialization();
        assert!(result.is_err());
    }

    const fn coin(amount: Amount) -> Coin<SuperGroupTestC1> {
        Coin::new(amount)
    }

    fn invalid_ratio_deserialization()
    -> Result<Ratio<Coin<SuperGroupTestC1>>, cosmwasm_std::StdError> {
        cosmwasm_std::from_json::<Ratio<Coin<SuperGroupTestC1>>>(&serialize_fraction())
    }

    fn serialize_fraction() -> Vec<u8> {
        cosmwasm_std::to_json_vec(&SimpleFraction::new(coin(5), coin(4))).unwrap()
    }

    mod into_coprime {
        use crate::{percent::Units as PercentUnits, ratio::SimpleFraction};

        #[test]
        fn into_coprime() {
            assert_eq!(SimpleFraction::new(1, 3), u_rational(2, 6))
        }

        #[test]
        fn into_coprime_primes() {
            assert_eq!(SimpleFraction::new(1009, 1061), u_rational(1009, 1061))
        }
        #[test]
        fn into_prime_big_coprime_values() {
            let max_even = PercentUnits::MAX - 1;
            assert_eq!(
                SimpleFraction::new(1, 2),
                u_rational(max_even / 2, max_even)
            )
        }
        #[test]
        fn into_prime_big_prime_values() {
            assert_eq!(
                SimpleFraction::new(u32::MAX, u32::MAX - 1),
                u_rational(u32::MAX, u32::MAX - 1)
            )
        }

        #[test]
        fn into_coprime_one() {
            assert_eq!(SimpleFraction::new(1, 1), u_rational(u32::MAX, u32::MAX));
        }

        fn u_rational(
            nominator: PercentUnits,
            denominator: PercentUnits,
        ) -> SimpleFraction<PercentUnits> {
            SimpleFraction::new(nominator, denominator)
        }
    }
}
