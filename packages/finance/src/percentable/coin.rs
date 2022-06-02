use std::ops::{Div, Mul};

use cosmwasm_std::{Coin, Uint256};

use crate::percent::Percent;

use super::Percentable;

impl Percentable for Coin {
    type Intermediate = Coin256;
    type Result = Self;
}

impl Percentable for &Coin {
    type Intermediate = Coin256;
    type Result = Coin;
}

#[derive(Debug, PartialEq)]
pub struct Coin256 {
    pub denom: String,
    pub amount: Uint256,
}

impl Mul<Percent> for &Coin {
    type Output = Coin256;

    fn mul(self, rhs: Percent) -> Self::Output {
        self.clone().mul(rhs)
    }
}

impl Mul<Percent> for Coin {
    type Output = Coin256;

    fn mul(self, rhs: Percent) -> Self::Output {
        Self::Output {
            denom: self.denom,
            amount: Uint256::from(self.amount).mul(Uint256::from(rhs.units())),
        }
    }
}

impl Div<Percent> for Coin256 {
    type Output = Coin;

    fn div(self, rhs: Percent) -> Self::Output {
        let amount256 = self.amount.div(Uint256::from(rhs.units()));
        Self::Output {
            denom: self.denom,
            amount: amount256.try_into().expect("Overflow computing percent"),
        }
    }
}

#[cfg(test)]
mod test {
    use cosmwasm_std::{Coin, Uint256};

    use crate::{percent::{Percent, test::test_of_are}, percentable::coin::Coin256};
    const DENOM: &str = "USDC";

    #[test]
    fn mul_coin_percent() {
        assert_eq!(
            Coin256 {
                denom: DENOM.into(),
                amount: Uint256::from(24680u32)
            },
            Coin::new(1234, DENOM) * Percent::from_percent(2)
        );
        assert_eq!(
            Coin256 {
                denom: DENOM.into(),
                amount: Uint256::from(u128::MAX) * Uint256::from(20u8)
            },
            Coin::new(u128::MAX, DENOM) * Percent::from_permille(20)
        );
    }

    #[test]
    fn div_coin256_percent() {
        assert_eq!(
            Coin::new(1234, DENOM),
            Coin256 {
                denom: DENOM.into(),
                amount: Uint256::from(246800u32)
            } / Percent::from_percent(20)
        );
        assert_eq!(
            Coin::new(u128::MAX, DENOM),
            Coin256 {
                denom: DENOM.into(),
                amount: Uint256::from(u128::MAX) * Uint256::from(200u8)
            } / Percent::from_percent(20)
        );
    }

    #[test]
    #[should_panic]
    fn div_overflow() {
        let _ = Coin256 {
            denom: DENOM.into(),
            amount: Uint256::from(u128::MAX) * Uint256::from(200u8),
        } / Percent::from_permille(199);
    }

    #[test]
    fn of_are() {
        test_of_are(10, Coin::new(100, DENOM), Coin::new(1, DENOM));
        test_of_are(
            1000,
            Coin::new(u128::MAX, DENOM),
            Coin::new(u128::MAX, DENOM),
        );
    }

    #[test]
    #[should_panic]
    fn of_overflow() {
        let max_amount = Coin::new(u128::MAX, DENOM);
        test_of_are(1001, max_amount.clone(), max_amount);
    }
    #[test]
    #[should_panic]
    fn are_overflow() {
        let max_amount = Coin::new(u128::MAX, DENOM);
        test_of_are(999, max_amount.clone(), max_amount);
    }
}
