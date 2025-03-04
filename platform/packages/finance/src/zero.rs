use sdk::cosmwasm_std::{Uint64, Uint128, Uint256, Uint512};

pub trait Zero {
    const ZERO: Self;
}

macro_rules! impl_zero {
    ($($type: ty),+ $(,)?) => {
        $(
            impl Zero for $type {
                const ZERO: Self = 0;
            }
        )+
    };
}

macro_rules! impl_cw_zero {
    ($($type: ty),+ $(,)?) => {
        $(
            impl Zero for $type {
                const ZERO: Self = Self::zero();
            }
        )+
    };
}

impl_zero!(
    i8, u8, i16, u16, i32, u32, i64, u64, i128, u128, isize, usize
);

impl_cw_zero!(Uint64, Uint128, Uint256, Uint512);
