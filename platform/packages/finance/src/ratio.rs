use std::fmt::Debug;

use serde::{Deserialize, Serialize};

use crate::{
    error::{Error, Result as FinanceResult},
    fraction::Fraction,
    fractionable::Fractionable,
    zero::Zero,
};

/// A part of something that is divisible
#[derive(Clone, Copy, Deserialize, Serialize)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug, PartialEq))]
#[serde(
    try_from = "Rational<U>",
    into = "Rational<U>",
    bound(
        serialize = "U: Clone + Serialize",
        deserialize = "U: Debug + Deserialize<'de> + PartialOrd + Zero"
    )
)]
pub struct Ratio<U>(Rational<U>);

impl<U> Ratio<U>
where
    U: Debug + PartialOrd + Zero,
{
    pub(crate) fn new(parts: U, total: U) -> Self {
        let obj = Self(Rational::new(parts, total));
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

impl<U> TryFrom<Rational<U>> for Ratio<U>
where
    U: Debug + PartialOrd + Zero,
{
    type Error = Error;

    fn try_from(rational: Rational<U>) -> Result<Self, Self::Error> {
        let res = Self::new(rational.nominator, rational.denominator);
        res.invariant_held()?;
        Ok(res)
    }
}

impl<U> From<Ratio<U>> for Rational<U>
where
    U: Clone,
{
    fn from(ratio: Ratio<U>) -> Rational<U> {
        ratio.0
    }
}

pub trait RatioLegacy<U> {
    fn parts(&self) -> U;
    fn total(&self) -> U;
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[cfg_attr(any(test, feature = "testing"), derive(PartialEq))]
#[serde(rename_all = "snake_case")]
pub struct Rational<U> {
    nominator: U,
    denominator: U,
}

impl<U> Rational<U>
where
    U: Debug + PartialEq<U> + Zero,
{
    #[track_caller]
    pub fn new(nominator: U, denominator: U) -> Self {
        debug_assert_ne!(denominator, Zero::ZERO);

        Self {
            nominator,
            denominator,
        }
    }
}

impl<U, T> Fraction<U> for Rational<T>
where
    Self: RatioLegacy<U>,
{
    #[track_caller]
    fn of<A>(&self, whole: A) -> A
    where
        A: Fractionable<U>,
    {
        whole.safe_mul(self)
    }
}

impl<U, T> RatioLegacy<U> for Rational<T>
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

#[cfg(test)]
mod test_ratio {

    use currency::test::SuperGroupTestC1;
    use sdk::cosmwasm_std;

    use crate::{
        coin::{Amount, Coin},
        ratio::{Ratio, Rational},
    };

    #[test]
    fn valid_invariant() {
        assert!(Ratio::new(1u32, 2u32).invariant_held().is_ok());
        assert!(Ratio::new(coin(9), coin(10)).invariant_held().is_ok());
        assert!(Ratio::new(coin(10), coin(10)).invariant_held().is_ok());
    }

    #[test]
    #[should_panic = "Parts must not exceed total"]
    fn invalid_variant() {
        let _ = Ratio::new(4u32, 3u32);
        let _ = Ratio::new(coin(10), coin(9));
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
    #[should_panic = "Parts must not exceed total"]
    fn serialize_deserialize_invalid() {
        let serialized = cosmwasm_std::to_json_vec(&Rational::new(coin(5), coin(4))).unwrap();
        let _ = cosmwasm_std::from_json::<Ratio<Coin<SuperGroupTestC1>>>(&serialized);
    }

    const fn coin(amount: Amount) -> Coin<SuperGroupTestC1> {
        Coin::new(amount)
    }
}
