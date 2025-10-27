use finance::{
    fractionable::{CommonDoublePrimitive, Fractionable, IntoMax, ToDoublePrimitive, TryFromMax},
    percent::Percent100,
};

use crate::feeders::PriceFeedersError;

#[derive(PartialEq, PartialOrd)]
pub struct Count(u32);

impl Count {
    pub const MAX: Self = Self(u32::MAX);

    pub const fn new(count: u32) -> Self {
        Self(count)
    }

    pub const fn count(&self) -> u32 {
        self.0
    }
}

impl TryFrom<usize> for Count {
    type Error = PriceFeedersError;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        value
            .try_into()
            .map_err(|_| Self::Error::MaxFeederCount {})
            .map(Self::new)
    }
}
impl CommonDoublePrimitive<Percent100> for Count {
    type CommonDouble = u64;
}

impl Fractionable<Percent100> for Count {}

impl IntoMax<u64> for Count {
    fn into_max(self) -> u64 {
        self.to_double()
    }
}

impl ToDoublePrimitive for Count {
    type Double = u64;

    fn to_double(&self) -> Self::Double {
        self.0.into()
    }
}

impl TryFromMax<u64> for Count {
    fn try_from_max(max: u64) -> Option<Self> {
        max.try_into().map(Self::new).ok()
    }
}
