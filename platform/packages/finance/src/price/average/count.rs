use std::ops::{Div, Rem};

use gcd::Gcd;

use crate::{
    coin::Amount,
    fraction::Unit as FractionUnit,
    fractionable::{CommonDoublePrimitive, Fractionable, IntoMax, ToDoublePrimitive, TryFromMax},
    percent::Percent100,
    ratio::Ratio,
    zero::Zero,
};

pub(super) type Unit = u32;
const ZERO: Unit = 0;
const ONE: Unit = 1;
const MAX: Unit = u32::MAX;

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Count(Unit);

impl Count {
    const ZERO: Self = Self::new(ZERO);
    pub(super) const ONE: Self = Self::new(ONE);
    pub const MAX: Self = Self::new(MAX);

    const fn new(count: Unit) -> Self {
        Self(count)
    }

    #[cfg(any(test, feature = "testing"))]
    pub const fn test_new(count: Unit) -> Self {
        Self::new(count)
    }

    /// Checks if [self] can be safely incremented
    pub fn check_increment(&self) -> Option<()> {
        if !self.is_zero() { Some(()) } else { None }
    }

    pub fn try_increment(self) -> Option<Self> {
        self.check_increment().map(|()| Self::new(self.0 + 1))
    }

    /// Converts [self] into a reciprocal fraction
    ///
    /// Returns [None] if the Count is zero
    pub fn try_into_reciprocal(self) -> Option<Ratio<Self>> {
        (!self.is_zero()).then(|| Ratio::new(Self::ONE, self))
    }

    pub(super) fn is_zero(&self) -> bool {
        self == &Self::ZERO
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

    fn to_double(self) -> Self::Double {
        self.0.into()
    }
}

impl TryFrom<usize> for Count {
    type Error = ();

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        value.try_into().map_err(|_| ()).map(Self::new)
    }
}

impl TryFromMax<<Count as ToDoublePrimitive>::Double> for Count {
    fn try_from_max(max: <Count as ToDoublePrimitive>::Double) -> Option<Self> {
        max.try_into().map(Self::new).ok()
    }
}

impl From<Count> for Amount {
    fn from(val: Count) -> Self {
        val.0.into()
    }
}

impl Zero for Count {
    const ZERO: Self = Self::ZERO;
}

#[cfg(test)]
mod test {
    use crate::ratio::Ratio;

    use super::{Count, Unit};

    #[test]
    fn try_into_reciprocal_nonzero() {
        let count = Count::test_new(4096);

        assert_eq!(
            Ratio::new(Count::ONE, count),
            count.try_into_reciprocal().unwrap()
        );
    }

    #[test]
    fn try_into_reciprocal_zero() {
        assert!(Count::ZERO.try_into_reciprocal().is_none())
    }

    #[test]
    fn can_increment_some() {
        assert_eq!(Some(()), Count::ZERO.check_increment());
        assert_eq!(Some(()), Count::test_new(100).check_increment());
        assert_eq!(Some(()), Count::test_new(65536).check_increment());
        assert_eq!(Some(()), Count::test_new(Unit::MAX - 1).check_increment());
    }

    #[test]
    fn can_increment_none() {
        assert!(Count::MAX.check_increment().is_none());
    }
}
