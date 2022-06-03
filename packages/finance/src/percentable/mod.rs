mod coin;
mod percent;
mod u128;
mod u64;

use cosmwasm_std::Fraction;

use crate::percent::Units;

pub trait Percentable {
    fn safe_mul<F>(self, fraction: &F) -> Self
    where
        F: Fraction<Units>;
}
