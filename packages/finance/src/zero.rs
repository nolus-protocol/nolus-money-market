use sdk::cosmwasm_std::{Uint128, Uint256, Uint512, Uint64};

pub trait Zero {
    const VALUE: Self;
}

macro_rules! impl_zero {
    ($($type: ty),+ $(,)?) => {
        $(
            impl Zero for $type {
                const VALUE: Self = 0;
            }
        )+
    };
}

macro_rules! impl_cw_zero {
    ($($type: ty),+ $(,)?) => {
        $(
            impl Zero for $type {
                const VALUE: Self = Self::zero();
            }
        )+
    };
}

impl_zero!(i8, u8, i16, u16, i32, u32, i64, u64, i128, u128);

impl_cw_zero!(Uint64, Uint128, Uint256, Uint512);
