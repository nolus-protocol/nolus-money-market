use std::fmt::Debug;

use serde::{Deserialize, Serialize};

use crate::{
    error::{Error, Result as FinanceResult},
    fraction::Fraction,
    fractionable::Fractionable,
    zero::Zero,
};

// /// A part of something that is divisible
#[derive(Clone, Deserialize, Serialize)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug, PartialEq))]
#[serde(
    try_from = "Rational<U>",
    into = "Rational<U>",
    rename_all = "snake_case"
)]
pub struct Ratio<U>(Rational<U>)
where
    U: Clone + Debug + PartialOrd + Zero;

impl<U> Ratio<U>
where
    U: Clone + Debug + PartialOrd + Zero,
{
    pub(crate) fn new(parts: U, total: U) -> FinanceResult<Self> {
        Error::broken_invariant_if::<Self>(parts > total, "Parts must not exceed total")
            .map(|()| Self(Rational::new(parts, total)))
    }
}

impl<U> TryFrom<Rational<U>> for Ratio<U>
where
    U: Clone + Debug + PartialOrd + Zero,
{
    type Error = Error;

    fn try_from(rational: Rational<U>) -> Result<Self, Self::Error> {
        Self::new(rational.nominator, rational.denominator)
    }
}

impl<U> From<Ratio<U>> for Rational<U>
where
    U: Clone + Debug + PartialOrd + Zero,
{
    fn from(ratio: Ratio<U>) -> Rational<U> {
        ratio.0
    }
}

pub trait RatioLegacy<U> {
    fn parts(&self) -> U;
    fn total(&self) -> U;
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "testing"), derive(PartialEq,))]
#[serde(rename_all = "snake_case")]
pub struct Rational<U> {
    nominator: U,
    denominator: U,
}

impl<U> Rational<U>
where
    U: Zero + Debug + PartialEq<U>,
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
    T: Zero + Copy + PartialEq + Into<U>,
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
    use std::fmt::{Debug, Display};

    use currency::test::SuperGroupTestC1;
    use sdk::cosmwasm_std;

    use crate::{
        coin::{Amount, Coin},
        ratio::{Ratio, Rational},
        zero::Zero,
    };

    #[test]
    fn ratio_new() {
        assert!(Ratio::new(1u32, 2u32).is_ok());
        assert!(Ratio::new(coin(9), coin(10)).is_ok());
        assert!(Ratio::new(coin(10), coin(10)).is_ok());
    }

    #[test]
    fn ratio_new_invalid() {
        let msg = "Parts must not exceed total";
        assert_err(Ratio::new(4u32, 3u32), msg);
        assert_err(Ratio::new(coin(10), coin(9)), msg);
    }

    #[test]
    fn serialize() {
        assert_eq!(
            r#"{"nominator":5,"denominator":7}"#,
            cosmwasm_std::to_json_string(&Ratio::new(5u32, 7u32).unwrap()).unwrap()
        );
        assert_eq!(
            r#"{"nominator":{"amount":"5"},"denominator":{"amount":"7"}}"#,
            cosmwasm_std::to_json_string(&Ratio::new(coin(5), coin(7)).unwrap()).unwrap()
        );
    }

    #[test]
    fn deserialize() {
        assert_eq!(
            Ratio::new(coin(5), coin(5)).unwrap(),
            cosmwasm_std::from_json(r#"{"nominator":{"amount":"5"},"denominator":{"amount":"5"}}"#)
                .unwrap()
        );
    }

    #[test]
    fn serialize_deserialize() {
        let serialized_1 = cosmwasm_std::to_json_vec(&Rational::new(coin(3), coin(4))).unwrap();
        assert_eq!(
            Ratio::new(coin(3), coin(4)).unwrap(),
            cosmwasm_std::from_json(&serialized_1).unwrap()
        );

        let serialized_2 =
            cosmwasm_std::to_json_vec(&Rational::new(coin(Amount::MAX), coin(Amount::MAX)))
                .unwrap();
        assert_eq!(
            Ratio::new(coin(Amount::MAX), coin(Amount::MAX)).unwrap(),
            cosmwasm_std::from_json(&serialized_2).unwrap()
        );
    }

    #[test]
    fn serialize_deserialize_invalid() {
        let serialized = cosmwasm_std::to_json_vec(&Rational::new(coin(5), coin(4))).unwrap();

        assert_err::<Coin<SuperGroupTestC1>, _>(
            cosmwasm_std::from_json(&serialized),
            "Parts must not exceed total",
        );
    }

    const fn coin(amount: Amount) -> Coin<SuperGroupTestC1> {
        Coin::new(amount)
    }

    fn assert_err<U, E>(res: Result<Ratio<U>, E>, msg: &str)
    where
        U: Clone + Debug + PartialOrd + Zero,
        E: Display,
    {
        let err = res.unwrap_err();
        assert!(
            err.to_string().contains(msg),
            "Error `{}` does not contain expected `{}`",
            err,
            msg
        );
    }
}
