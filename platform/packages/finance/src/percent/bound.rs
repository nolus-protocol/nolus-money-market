use std::fmt::{Debug, Display, Formatter, Result as FmtResult, Write};

#[cfg(any(test, feature = "testing"))]
use std::ops::{Add, Sub};

use serde::{Deserialize, Serialize};

use crate::{error::Error, fraction::Unit, percent::permilles::Permilles};

use super::Units;

/// Represents a percentage value used in domain logic.
/// The const generic parameter `UPPER_BOUND` defines the maximum allowed percentage (inclusive).
///
/// Values are stored in `Permilles`.
/// For example, `UPPER_BOUND = 1000` represents 100%, and a value of `700` represents 70%.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(into = "Permilles", try_from = "Permilles")]
pub struct BoundPercent<const UPPER_BOUND: Units>(Permilles);

impl<const UPPER_BOUND: Units> BoundPercent<UPPER_BOUND> {
    pub const ZERO: Self =
        Self::try_from_permille(Permilles::ZERO).expect("0% is a valid instance");
    pub const MAX: Self = Self::try_from_permille(Permilles::new(UPPER_BOUND))
        .expect("UPPER_BOUND/UPPER_BOUND is a valid BoundPercent");
    pub const PRECISION: Self =
        Self::try_from_permille(Permilles::PRECISION).expect("0.1% is a valid instance");

    const UNITS_TO_PERCENT_RATIO: Units = 10;

    #[cfg(any(test, feature = "testing"))]
    pub const fn from_percent(percent: u32) -> Self {
        let units = percent
            .checked_mul(Self::UNITS_TO_PERCENT_RATIO)
            .expect("Percent value exceeds allowed upper bound");
        Self::from_permille(units)
    }

    #[cfg(any(test, feature = "testing"))]
    pub const fn from_permille(units: Units) -> Self {
        Self::try_from_permille(Permilles::new(units))
            .expect("Permille value exceeds allowed upper bound")
    }

    const fn try_from_permille(permille: Permilles) -> Option<Self> {
        if permille.units() <= UPPER_BOUND {
            Some(Self(permille))
        } else {
            None
        }
    }

    pub fn display_primitive(&self) -> String {
        format!("{}", self.0.to_primitive())
    }

    // Cannot be const because const impl of PartialEq is not available.
    pub fn is_zero(&self) -> bool {
        self == &Self::ZERO
    }

    pub const fn checked_add(self, other: Self) -> Option<Self> {
        if let Some(res) = self.0.checked_add(other.0) {
            Self::try_from_permille(res)
        } else {
            None
        }
    }

    pub const fn checked_sub(self, other: Self) -> Option<Self> {
        if let Some(res) = self.0.checked_sub(other.0) {
            Self::try_from_permille(res)
        } else {
            None
        }
    }
}

impl<const UPPER_BOUND: Units> From<BoundPercent<UPPER_BOUND>> for Permilles {
    fn from(percent: BoundPercent<UPPER_BOUND>) -> Self {
        percent.0
    }
}

impl<const UPPER_BOUND: Units> TryFrom<Permilles> for BoundPercent<UPPER_BOUND> {
    type Error = Error;

    fn try_from(permille: Permilles) -> Result<Self, Self::Error> {
        Self::try_from_permille(permille).ok_or(Error::UpperBoundCrossed {
            bound: UPPER_BOUND,
            value: permille,
        })
    }
}

impl<const UPPER_BOUND: Units> Display for BoundPercent<UPPER_BOUND> {
    #[track_caller]
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let whole = (Permilles::from(*self).to_primitive()) / Self::UNITS_TO_PERCENT_RATIO;
        let (no_fraction, overflow) = whole.overflowing_mul(Self::UNITS_TO_PERCENT_RATIO);
        debug_assert!(!overflow);
        let (fractional, overflow) =
            (Permilles::from(*self).to_primitive()).overflowing_sub(no_fraction);
        debug_assert!(!overflow);

        f.write_fmt(format_args!("{whole}"))?;
        if fractional != Units::default() {
            f.write_fmt(format_args!(".{fractional}"))?;
        }
        f.write_char('%')?;
        Ok(())
    }
}

#[cfg(any(test, feature = "testing"))]
impl<const UPPER_BOUND: Units> Add for BoundPercent<UPPER_BOUND> {
    type Output = Self;

    #[track_caller]
    fn add(self, rhs: Self) -> Self {
        self.checked_add(rhs).expect("attempt to add with overflow")
    }
}

#[cfg(any(test, feature = "testing"))]
impl<const UPPER_BOUND: Units> Sub for BoundPercent<UPPER_BOUND> {
    type Output = Self;

    #[track_caller]
    fn sub(self, rhs: Self) -> Self {
        self.checked_sub(rhs)
            .expect("attempt to subtract with overflow")
    }
}

#[cfg(test)]
mod test {
    use sdk::cosmwasm_std;

    use crate::{
        fraction::Fraction,
        percent::{
            HUNDRED, Percent, Percent100, Units, bound::BoundPercent, permilles::Permilles, test,
        },
        rational::Rational,
        test::coin,
    };

    #[test]
    fn serialize() {
        assert_eq!(
            r#"650"#,
            cosmwasm_std::to_json_string(&Percent100::from_permille(650)).unwrap()
        );
        assert_eq!(
            r#"2001"#,
            cosmwasm_std::to_json_string(&Percent::from_permille(2001)).unwrap()
        );
    }

    #[test]
    fn deserialize() {
        assert_eq!(
            Percent100::from_permille(999),
            cosmwasm_std::from_json(r#"999"#).unwrap()
        );
        assert_eq!(
            Percent::from_permille(4000),
            cosmwasm_std::from_json(r#"4000"#).unwrap()
        );
    }

    #[test]
    fn deserialize_upper_bound_crossed() {
        assert_err(
            cosmwasm_std::from_json::<Percent100>("1001"),
            "Upper bound has been crossed",
        );

        let too_big = (u64::from(Units::MAX) + 1u64).to_string();
        assert_err(cosmwasm_std::from_json::<Percent>(&too_big), "Invalid");
    }

    #[test]
    fn serialize_deserialize() {
        let percent100 = Percent100::from_permille(250);
        let serialized = cosmwasm_std::to_json_vec(&percent100).unwrap();
        assert_eq!(percent100, cosmwasm_std::from_json(&serialized).unwrap());

        let percent = Percent::from_permille(1001);
        let serialized = cosmwasm_std::to_json_vec(&percent).unwrap();
        assert_eq!(percent, cosmwasm_std::from_json(&serialized).unwrap());
    }

    #[test]
    fn test_try_from_permille() {
        assert_eq!(try_from_permille(0), Some(Percent::ZERO));
        assert_eq!(try_from_permille(0), Some(Percent100::ZERO));

        assert_eq!(try_from_permille(100), Some(test::percent(100)));
        assert_eq!(try_from_permille(100), Some(test::percent100(100)));

        assert_eq!(try_from_permille(HUNDRED), Some(test::percent(HUNDRED)));
        assert_eq!(try_from_permille(HUNDRED), Some(Percent100::MAX));

        assert_eq!(try_from_permille(1001), Some(test::percent(1001)));
        assert_eq!(try_from_permille(1001), Option::<Percent100>::None);

        assert_eq!(try_from_permille(Units::MAX), Some(Percent::MAX));
        assert_eq!(try_from_permille(Units::MAX), Option::<Percent100>::None);
    }

    #[test]
    fn test_zero() {
        let zero_amount = coin::coin1(0);
        assert_eq!(zero_amount, Percent100::ZERO.of(coin::coin1(10)));
        assert_eq!(zero_amount, Percent::ZERO.of(coin::coin1(10)).unwrap())
    }

    #[test]
    fn test_percent100_max() {
        let amount = coin::coin1(123);
        assert_eq!(amount, Percent100::MAX.of(amount));
    }

    #[test]
    fn test_percent_max() {
        let amount = coin::coin1(1);

        assert_eq!(
            coin::coin1((Units::MAX / HUNDRED).into()),
            Percent::MAX.of(amount).unwrap()
        );
    }

    #[test]
    fn test_display_primitive() {
        assert_eq!("304", Percent100::from_permille(304).display_primitive());
        assert_eq!("0", Percent100::ZERO.display_primitive());
    }

    #[test]
    fn checked_add() {
        assert_eq!(
            test::percent100(40),
            test::percent100(25) + (test::percent100(15))
        );
        assert_eq!(
            test::percent100(39),
            test::percent100(0) + (test::percent100(39))
        );
        assert_eq!(
            test::percent100(39),
            test::percent100(39) + (test::percent100(0))
        );
        assert_eq!(
            Percent100::MAX,
            test::percent100(999) + (test::percent100(1))
        );
    }

    #[test]
    fn add_overflow() {
        assert!(Percent100::MAX.checked_add(test::percent100(1)).is_none());
        assert!(
            test::percent(Units::MAX)
                .checked_add(test::percent(1))
                .is_none()
        );
    }

    #[test]
    fn sub() {
        assert_eq!(
            test::percent100(67),
            test::percent100(79) - (test::percent100(12))
        );
        assert_eq!(
            test::percent100(0),
            test::percent100(34) - (test::percent100(34))
        );
        assert_eq!(
            test::percent100(39),
            test::percent100(39) - (test::percent100(0))
        );
        assert_eq!(test::percent100(990), test::percent100(10).complement());
        assert_eq!(test::percent100(0), test::percent100(HUNDRED).complement());
    }

    #[test]
    fn sub_overflow() {
        assert!(
            test::percent100(34)
                .checked_sub(test::percent100(35))
                .is_none()
        )
    }

    #[test]
    fn display() {
        test_display("0%", 0);
        test_display("0.1%", 1);
        test_display("0.4%", 4);
        test_display("1%", 10);
        test_display("1.9%", 19);
        test_display("9%", 90);
        test_display("10.1%", 101);
        test_display("100%", HUNDRED);
    }

    fn assert_err<P>(r: Result<P, cosmwasm_std::StdError>, msg: &str)
    where
        P: std::fmt::Debug,
    {
        assert!(r.expect_err("expected an error").to_string().contains(msg));
    }

    fn test_display(exp: &str, permilles: Units) {
        assert_eq!(exp, format!("{}", test::percent100(permilles)));
    }

    fn try_from_permille<const UPPER_BOUND: Units>(
        units: Units,
    ) -> Option<BoundPercent<UPPER_BOUND>> {
        BoundPercent::try_from_permille(Permilles::new(units))
    }
}
