use cosmwasm_std::{Fraction, Uint256};

use crate::coin::Coin;

use super::Fractionable;

impl<U, C> Fractionable<U> for Coin<C>
where
    Uint256: From<U>,
    U: PartialEq,
{
    fn safe_mul<F>(self, fraction: &F) -> Self
    where
        F: Fraction<U>,
    {
        Self::new(self.amount().safe_mul(fraction))
    }
}

#[cfg(test)]
mod test {
    use crate::{percent::test::{test_are, test_of, test_of_are}, coin::{Usdc, Coin}};

    #[test]
    fn of_are() {
        test_of_are(10, usdc(100), usdc(1));
        test_of(11, usdc(100), usdc(1));
        test_are(11, usdc(1), usdc(90));
        test_of(110, usdc(100), usdc(11));
        test_are(110, usdc(11), usdc(100));
        test_of(12, usdc(100), usdc(1));
        test_are(12, usdc(1), usdc(83));
        test_of(18, usdc(100), usdc(1));
        test_are(18, usdc(1), usdc(55));
        test_of(18, usdc(120), usdc(2));
        test_are(18, usdc(2), usdc(111));
        test_of_are(
            1000,
            usdc(u128::MAX),
            usdc(u128::MAX),
        );
    }

    #[test]
    #[should_panic]
    fn of_overflow() {
        let max_amount = usdc(u128::MAX);
        test_of(1001, max_amount, max_amount);
    }
    #[test]
    #[should_panic]
    fn are_overflow() {
        let max_amount = usdc(u128::MAX);
        test_are(999, max_amount, max_amount);
    }

    fn usdc(amount: u128) -> Coin<Usdc> {
        Coin::new(amount)
    }
}
