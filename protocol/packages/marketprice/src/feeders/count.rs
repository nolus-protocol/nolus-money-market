use std::ops::{Div, Rem};

use gcd::Gcd;

use finance::{
    fraction::Unit as FractionUnit,
    fractionable::{CommonDoublePrimitive, Fractionable, IntoMax, ToDoublePrimitive, TryFromMax},
    percent::Percent100,
    ratio::Ratio,
    zero::Zero,
};

use crate::feeders::PriceFeedersError;

pub type Unit = u32;
const ZERO: Unit = 0;
const ONE: Unit = 1;
const MAX: Unit = u32::MAX;

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Count(Unit);

impl Count {
    const ZERO: Self = Self(ZERO);
    pub const MAX: Self = Self(MAX);

    const fn new(count: Unit) -> Self {
        Self(count)
    }

    #[cfg(any(test, feature = "testing"))]
    pub const fn new_test(count: Unit) -> Self {
        Self::new(count)
    }

    /// Checks if [self] can be safely incremented
    pub fn check_increment(&self) -> Result<(), PriceFeedersError> {
        if self != &Self::MAX {
            Ok(())
        } else {
            Err(PriceFeedersError::MaxFeederCount {})
        }
    }

    /// Converts [self] into a reciproral fraction
    ///
    /// Returns [None] if the Count is zero
    pub fn try_into_reciproral(self) -> Option<Ratio<Unit>> {
        (self != Self::ZERO).then(|| Ratio::new(ONE, self.0))
    }
}

impl CommonDoublePrimitive<Percent100> for Count {
    type CommonDouble = <Count as ToDoublePrimitive>::Double;
}

impl Fractionable<Percent100> for Count {}

impl FractionUnit for Count {
    type Times = Unit;

    fn gcd<U>(self, other: U) -> Self::Times
    where
        U: FractionUnit<Times = Self::Times>,
    {
        Gcd::gcd(self.0, other.to_primitive())
    }

    fn scale_down(self, scale: Self::Times) -> Self {
        debug_assert_ne!(scale, Self::Times::ZERO);

        Self::new(self.0.div(scale))
    }

    fn modulo(self, scale: Self::Times) -> Self::Times {
        debug_assert_ne!(scale, Self::Times::ZERO);

        self.0.rem(scale)
    }

    fn to_primitive(self) -> Self::Times {
        self.0
    }
}

impl IntoMax<<Count as CommonDoublePrimitive<Percent100>>::CommonDouble> for Count {
    fn into_max(self) -> <Count as ToDoublePrimitive>::Double {
        self.to_double()
    }
}

impl ToDoublePrimitive for Count {
    type Double = u64;

    fn to_double(&self) -> Self::Double {
        self.0.into()
    }
}

impl TryFrom<usize> for Count {
    type Error = PriceFeedersError;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        value
            .try_into()
            .map_err(Self::Error::FeederCountExceeded)
            .map(Self::new)
    }
}

impl TryFromMax<<Count as ToDoublePrimitive>::Double> for Count {
    fn try_from_max(max: <Count as ToDoublePrimitive>::Double) -> Option<Self> {
        max.try_into().map(Self::new).ok()
    }
}

impl Zero for Count {
    const ZERO: Self = Self::ZERO;
}

#[cfg(test)]
mod test {

    use finance::ratio::Ratio;

    use crate::feeders::count::Unit;

    use super::Count;

    #[test]
    fn try_into_reciproral_nonzero() {
        let count = 4096;
        assert_eq!(
            Ratio::new(1, count),
            Count::new_test(count).try_into_reciproral().unwrap()
        );
    }

    #[test]
    fn try_into_reciproral_zero() {
        assert!(Count::ZERO.try_into_reciproral().is_none())
    }

    #[test]
    fn can_increment_some() {
        assert_eq!(Ok(()), Count::ZERO.check_increment());
        assert_eq!(Ok(()), Count::new_test(100).check_increment());
        assert_eq!(Ok(()), Count::new_test(65536).check_increment());
        assert_eq!(Ok(()), Count::new_test(Unit::MAX - 1).check_increment());
    }

    #[test]
    fn can_increment_none() {
        assert!(Count::MAX.check_increment().is_err());
    }
}
