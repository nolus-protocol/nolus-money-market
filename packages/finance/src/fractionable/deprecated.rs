use cosmwasm_std::{Uint128, Uint256};

use super::HigherRank;

impl<T> HigherRank<T> for Uint128 {
    type Type = Uint256;

    type Intermediate = Self;
}
