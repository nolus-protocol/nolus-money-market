use std::{
    fmt::Debug,
    ops::{Div, Mul}
};

use cosmwasm_std::{Uint128, Uint256};

pub trait Integer
where
    Self: Mul<Output = Self> + Div<Output = Self> + Into<<Self as Integer>::Double>,
    <<Self as Integer>::Double as TryInto<Self>>::Error: Debug,
{
    type Double: Mul<Output = <Self as Integer>::Double> + Div<Output = <Self as Integer>::Double> + TryInto<Self>;
}

macro_rules! impl_integer {
    ($int: ty, $double: ty $(,)?) => {
        impl Integer for $int {
            type Double = $double;
        }
    };
    ($int: ty, $double: ty, $($rest: ty),+ $(,)?) => {
        impl_integer!($int, $double);

        impl_integer!($double, $($rest),+);
    };
}

impl_integer![u8, u16, u32, u64, u128];

impl_integer![Uint128, Uint256];

pub trait IndirectInto<T, U>
where
    Self: Into<T>,
    T: Into<U>,
{}

impl<T, U, V> IndirectInto<U, V> for T
where
    T: Into<U>,
    U: Into<V>,
{}

pub struct Ratio<T>
where
    T: Copy,
{
    nominator: T,
    denominator: T,
}

impl<T> Ratio<T>
where
    T: Copy,
{
    pub fn new(nominator: T, denominator: T) -> Self {
        Self {
            nominator,
            denominator,
        }
    }

    pub fn of<T2Int, U, U2Int, Int>(&self, value: U) -> U
    where
        T: IndirectInto<T2Int, Int> + Into<T2Int>,
        T2Int: Into<Int>,
        U: IndirectInto<U2Int, Int> + Into<U2Int>,
        U2Int: Into<Int> + TryInto<U>,
        <U2Int as TryInto<U>>::Error: Debug,
        Int: Integer + TryInto<U2Int>,
        <Int as TryInto<U2Int>>::Error: Debug,
        <<Int as Integer>::Double as TryInto<Int>>::Error: Debug,
    {
        TryInto::<U>::try_into(
            TryInto::<U2Int>::try_into(
                TryInto::<Int>::try_into(
                    (
                        Into::<<Int as Integer>::Double>::into(
                            Into::<Int>::into(Into::<T2Int>::into(self.nominator))
                        ) * Into::<<Int as Integer>::Double>::into(
                            Into::<Int>::into(Into::<U2Int>::into(value))
                        )
                    ) / Into::<<Int as Integer>::Double>::into(
                        Into::<Int>::into(Into::<T2Int>::into(self.denominator))
                    )
                ).expect("Integer overflow occurred during ratio calculation!")
            ).unwrap()
        ).unwrap()
    }
}
