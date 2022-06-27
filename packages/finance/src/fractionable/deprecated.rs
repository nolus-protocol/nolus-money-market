use cosmwasm_std::{Uint128, Uint256, Coin};

use crate::ratio::Ratio;

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
    fn safe_mul<R>(self, ratio: &R) -> Self
    where
        R: Ratio<U>,
    {
        Self {
            amount: self.amount.safe_mul(ratio),
            denom: self.denom,
        }
    }
}
