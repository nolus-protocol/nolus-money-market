use crate::{coin::Coin, duration::Duration};

use super::Fractionable;

impl<C> Fractionable<Coin<C>> for Duration {
    type MaxRank = u128;
}

#[cfg(test)]
mod tests {
    use currency::test::SuperGroupTestC1;

    use crate::{coin::Coin, duration::Duration, ratio::SimpleFraction};

    #[test]
    fn lossy_mul() {
        let d = Duration::from_secs(10);
        let res = SimpleFraction::new(
            Coin::<SuperGroupTestC1>::new(10),
            Coin::<SuperGroupTestC1>::new(20),
        )
        .lossy_mul(d)
        .unwrap();
        assert_eq!(Duration::from_secs(5), res);
    }

    #[test]
    fn lossy_mul_max() {
        let d = Duration::from_secs(10);
        let res_x = SimpleFraction::new(
            Coin::<SuperGroupTestC1>::new(u128::MAX),
            Coin::<SuperGroupTestC1>::new(u128::MAX / 2),
        );
        let res = res_x.lossy_mul(d).unwrap();
        assert_eq!(Duration::from_secs(20), res);
    }
}
