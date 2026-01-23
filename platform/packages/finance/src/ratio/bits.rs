use crate::{
    coin::{Amount, DoubleCoinPrimitive},
    percent::Units as PercentUnits,
};

pub trait Bits {
    const BITS: u32;

    fn leading_zeros(self) -> u32;
}

impl Bits for PercentUnits {
    const BITS: u32 = Self::BITS;

    fn leading_zeros(self) -> u32 {
        self.leading_zeros()
    }
}

impl Bits for Amount {
    const BITS: u32 = Self::BITS;

    fn leading_zeros(self) -> u32 {
        self.leading_zeros()
    }
}

impl Bits for DoubleCoinPrimitive {
    const BITS: u32 = Self::BITS;

    fn leading_zeros(self) -> u32 {
        self.leading_zeros()
    }
}
