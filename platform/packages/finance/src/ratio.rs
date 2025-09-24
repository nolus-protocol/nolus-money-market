use std::{fmt::Debug, ops::Div};

use serde::{Deserialize, Serialize};

use crate::{
    error::{Error, Result as FinanceResult},
    fraction::{Fraction, Unit as FractionUnit},
    fractionable::{FractionOutcome, Fractionable, ToDoublePrimitive, checked_mul::CheckedMul},
    rational::Rational,
    zero::Zero,
};

/// A part of something that is divisible
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
        A: Fractionable<U>,
    {
        self.0
            .of(whole)
            .expect("Ratio is a part of a whole, multiplication cannot overflow")
    }
}

pub trait RatioLegacy<U> {
    fn parts(&self) -> U;
    fn total(&self) -> U;
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[cfg_attr(any(test, feature = "testing"), derive(PartialEq))]
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

        Self {
            nominator,
            denominator,
        }
    }

    pub fn checked_mul<M>(&self, rhs: M) -> Option<M>
    where
        U: ToDoublePrimitive,
        M: FractionOutcome<U>,
    {
        if self.nominator == self.denominator {
            Some(rhs)
        } else {
            let nominator_max = M::into_max_other(self.nominator);
            let rhs_max = rhs.into_max_self();
            let denominator_max = M::into_max_other(self.denominator);

            nominator_max
                .checked_mul(rhs_max)
                .map(|product| product.div(denominator_max))
                .and_then(M::try_from_max)
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

impl<U, T> Rational<U> for SimpleFraction<T>
where
    Self: RatioLegacy<U>,
{
    fn of<A>(&self, whole: A) -> Option<A>
    where
        A: Fractionable<U>,
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
        let serialized = cosmwasm_std::to_json_vec(&SimpleFraction::new(coin(5), coin(4))).unwrap();
        cosmwasm_std::from_json::<Ratio<Coin<SuperGroupTestC1>>>(&serialized)
    }
}
