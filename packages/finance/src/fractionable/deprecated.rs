use cosmwasm_std::{Uint128, Uint256, Coin, Fraction};

use super::{Integer, Fractionable};

// #[deprecated = "Migrate to using u128 or finance::coin::Coin"]
impl Integer for Uint128 {
    type SameBitsInteger = Self;
    type DoubleInteger = Uint256;
}

impl<U> Fractionable<U> for Coin
where
    Uint256: From<U>,
    U: PartialEq,
{
    fn safe_mul<F>(self, fraction: &F) -> Self
    where
        F: Fraction<U>,
    {
        Self {
            amount: self.amount.safe_mul(fraction),
            denom: self.denom,
        }
    }
}
