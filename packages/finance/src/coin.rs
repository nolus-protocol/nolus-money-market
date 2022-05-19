use cosmwasm_std::{Coin, Uint128};

pub fn sub_amount(from: Coin, amount: Uint128) -> Coin {
    Coin {
        amount: from.amount - amount,
        denom: from.denom,
    }
}

pub fn add_coin(to: Coin, other: Coin) -> Coin {
    debug_assert!(to.denom == other.denom);
    Coin {
        amount: to.amount + other.amount,
        denom: to.denom,
    }
}
