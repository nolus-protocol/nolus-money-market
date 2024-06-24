use std::{
    error::Error,
    fmt::Debug,
    mem::size_of,
    ops::{Div, Mul},
};

use sdk::cosmwasm_std::{Uint128, Uint256, Uint512, Uint64};

pub fn total_of<T, U>(
    whole: T,
    ratio: Ratio<U>,
) -> Result<Result<T, <U as TryInto<T>>::Error>, <U as Unit>::Error>
where
    T: Into<U>,
    U: Unit + TryInto<T>,
{
    let whole = whole.into().into_doubled();

    let parts = ratio.parts.into_doubled();

    let total = ratio.total.into_doubled();

    U::try_from_doubled((whole * parts) / total).map(TryInto::try_into)
}

pub struct Ratio<T>
where
    T: Unit,
{
    parts: T,
    total: T,
}

impl<T> Ratio<T>
where
    T: Unit,
{
    pub const fn new(parts: T, total: T) -> Self {
        const {
            if size_of::<T>() * 2 > size_of::<T::DoubledCapacity>() {
                panic!("Unit used unit type is incorrectly defined!")
            }
        }

        Self { parts, total }
    }

    #[inline]
    pub const fn parts(&self) -> &T {
        &self.parts
    }

    #[inline]
    pub const fn total(&self) -> &T {
        &self.total
    }

    pub fn map<U>(self) -> Ratio<U>
    where
        T: Unit + Into<U>,
        U: Unit,
    {
        Ratio {
            parts: self.parts.into(),
            total: self.total.into(),
        }
    }

    pub fn try_map<U>(self) -> Result<Ratio<U>, <T as TryInto<U>>::Error>
    where
        T: Unit + TryInto<U>,
        U: Unit,
    {
        self.parts
            .try_into()
            .and_then(|parts| self.total.try_into().map(|total| Ratio { parts, total }))
    }
}

impl<T, U> Mul<U> for Ratio<T>
where
    T: Unit + TryInto<U>,
    <T as TryInto<U>>::Error: Debug,
    U: Into<T>,
{
    type Output = U;

    #[inline]
    fn mul(self, rhs: U) -> Self::Output {
        total_of(rhs, self).unwrap().unwrap()
    }
}

pub trait Unit: Sized {
    type DoubledCapacity: Mul<Output = Self::DoubledCapacity> + Div<Output = Self::DoubledCapacity>;

    type Error: Error;

    fn into_doubled(self) -> Self::DoubledCapacity;

    fn try_from_doubled(value: Self::DoubledCapacity) -> Result<Self, Self::Error>;
}

macro_rules! define_simple_units {
    ($($from:ty => $to:ty),+ $(,)?) => {
        $(
            impl Unit for $from {
                type DoubledCapacity = $to;

                type Error = <$to as TryInto<$from>>::Error;

                #[inline]
                fn into_doubled(self) -> Self::DoubledCapacity {
                    self.into()
                }

                #[inline]
                fn try_from_doubled(value: Self::DoubledCapacity) -> Result<Self, Self::Error> {
                    value.try_into()
                }
            }
        )+
    };
}

define_simple_units![
    u8 => u16,
    u16 => u32,
    u32 => u64,
    u64 => u128,
    Uint64 => Uint128,
    Uint128 => Uint256,
    Uint256 => Uint512,
];

impl Unit for u128 {
    type DoubledCapacity = Uint256;

    type Error = <Uint256 as TryInto<Uint128>>::Error;

    #[inline]
    fn into_doubled(self) -> Self::DoubledCapacity {
        self.into()
    }

    #[inline]
    fn try_from_doubled(value: Self::DoubledCapacity) -> Result<Self, Self::Error> {
        value.try_into().map(Uint128::into)
    }
}
