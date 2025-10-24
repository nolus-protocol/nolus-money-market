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
