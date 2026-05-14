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

impl_zero!(
    i8, u8, i16, u16, i32, u32, i64, u64, i128, u128, isize, usize
);
