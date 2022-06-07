use cosmwasm_std::{Coin, Fraction, Uint256};

use super::Fractionable;

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

#[cfg(test)]
mod test {
    use cosmwasm_std::Coin;

    use crate::percent::test::{test_are, test_of, test_of_are};
    const DENOM: &str = "USDC";

    #[test]
    fn of_are() {
        test_of_are(10, Coin::new(100, DENOM), Coin::new(1, DENOM));
        test_of(11, Coin::new(100, DENOM), Coin::new(1, DENOM));
        test_are(11, Coin::new(1, DENOM), Coin::new(90, DENOM));
        test_of(110, Coin::new(100, DENOM), Coin::new(11, DENOM));
        test_are(110, Coin::new(11, DENOM), Coin::new(100, DENOM));
        test_of(12, Coin::new(100, DENOM), Coin::new(1, DENOM));
        test_are(12, Coin::new(1, DENOM), Coin::new(83, DENOM));
        test_of(18, Coin::new(100, DENOM), Coin::new(1, DENOM));
        test_are(18, Coin::new(1, DENOM), Coin::new(55, DENOM));
        test_of(18, Coin::new(120, DENOM), Coin::new(2, DENOM));
        test_are(18, Coin::new(2, DENOM), Coin::new(111, DENOM));
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
        test_of(1001, max_amount.clone(), max_amount);
    }
    #[test]
    #[should_panic]
    fn are_overflow() {
        let max_amount = Coin::new(u128::MAX, DENOM);
        test_are(999, max_amount.clone(), max_amount);
    }
}
