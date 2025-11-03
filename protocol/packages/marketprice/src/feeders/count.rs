use finance::{
    fractionable::{CommonDoublePrimitive, Fractionable, IntoMax, ToDoublePrimitive, TryFromMax},
    percent::Percent100,
};

use crate::feeders::PriceFeedersError;

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Count(u32);

impl Count {
    pub const MAX: Self = Self(u32::MAX);

    pub const fn new(count: u32) -> Self {
        Self(count)
    }

    pub fn can_increment(&self) -> Option<()> {
        (self != &Self::MAX).then_some(())
    }
}

impl TryFrom<usize> for Count {
    type Error = PriceFeedersError;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        value
            .try_into()
            .map_err(Self::Error::MaxFeederCount)
            .map(Self::new)
    }
}
impl CommonDoublePrimitive<Percent100> for Count {
    type CommonDouble = <Count as ToDoublePrimitive>::Double;
}

impl Fractionable<Percent100> for Count {}

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

impl TryFromMax<<Count as ToDoublePrimitive>::Double> for Count {
    fn try_from_max(max: <Count as ToDoublePrimitive>::Double) -> Option<Self> {
        max.try_into().map(Self::new).ok()
    }
}
