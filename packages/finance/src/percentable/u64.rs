use super::Integer;

impl Integer for u64 {
    type SameBitsInteger = Self;
    type DoubleInteger = u128;
}

#[cfg(test)]
mod test {
    use crate::percent::test::{test_are, test_of, test_of_are};

    #[test]
    fn of_are() {
        test_of_are(100, 50u64, 5u64);
        test_of_are(100, 5000u64, 500u64);
        test_of_are(101, 5000u64, 505u64);
        test_of_are(200, 50u64, 10u64);
        test_of(0, 120u64, 0u64);
        test_of_are(1, 1000u64, 1u64);
        test_of_are(1, 0u64, 0u64);
        test_of_are(200, 0u64, 0u64);
        test_of_are(1200, 50u64, 60u64);
        test_of_are(12, 500u64, 6u64);
        test_of_are(1000, u64::MAX, u64::MAX);
    }

    #[test]
    #[should_panic]
    fn of_overflow() {
        test_of(2001, u64::MAX / 2, u64::MAX);
    }

    #[test]
    #[should_panic]
    fn are_overflow() {
        test_are(999, u64::MAX, u64::MAX);
    }

    #[test]
    #[should_panic]
    fn are_div_zero() {
        test_are(0, u64::MAX, u64::MAX);
    }
}
