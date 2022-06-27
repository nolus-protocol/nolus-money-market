use cosmwasm_std::{Uint256, Uint128};

use super::Integer;

impl Integer for u128 {
    type SameBitsInteger = Uint128;
    type DoubleInteger = Uint256;
}

#[cfg(test)]
mod test {

    use crate::percent::test::{test_are, test_of, test_of_are};

    #[test]
    fn of_are() {
        test_of_are(1200, 50u128, 60u128);
        test_of_are(12, 500u128, 6u128);
        test_of_are(1000, u128::MAX, u128::MAX);
    }

    #[test]
    #[should_panic]
    fn of_overflow() {
        test_of(1001, u128::MAX, u128::MAX);
    }

    #[test]
    #[should_panic]
    fn are_overflow() {
        test_are(999, u128::MAX, u128::MAX);
    }
    #[test]
    #[should_panic]
    fn are_div_zero() {
        test_are(0, u128::MAX, u128::MAX);
    }
}
