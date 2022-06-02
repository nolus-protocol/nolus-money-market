use std::ops::{Div, Mul};

use cosmwasm_std::{Uint128, Uint256};

use crate::{percent::Percent, percentable::Percentable};

impl Percentable for Uint128 {
    type Intermediate = Uint256;
    type Result = Self;
}
impl Mul<Percent> for Uint128 {
    type Output = Uint256;

    fn mul(self, rhs: Percent) -> Self::Output {
        Self::Output::from(self).mul(Self::Output::from(rhs.units()))
    }
}
impl Div<Percent> for Uint256 {
    type Output = Uint128;

    fn div(self, rhs: Percent) -> Self::Output {
        let out128 = self.div(Self::from(rhs.units()));
        out128.try_into().expect("Overflow")
    }
}

#[cfg(test)]
mod test {
    use cosmwasm_std::Uint128;

    use crate::percent::test::test_of_are;

    #[test]
    fn of_are() {
        test_of_are(1200, Uint128::from(50u32), Uint128::from(60u8));
        test_of_are(12, Uint128::from(500u16), Uint128::from(6u8));
        test_of_are(1000, Uint128::MAX, Uint128::MAX);
    }

    #[test]
    #[should_panic]
    fn of_overflow() {
        test_of_are(1001, Uint128::MAX, Uint128::MAX);
    }

    #[test]
    #[should_panic]
    fn are_overflow() {
        test_of_are(999, Uint128::MAX, Uint128::MAX);
    }
}
