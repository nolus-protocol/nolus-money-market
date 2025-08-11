use bnum::types::U256;
use sdk::cosmwasm_std::{Uint128, Uint256};

use crate::{
    coin::Coin,
    duration::{Duration, Units},
    fractionable::{Fractionable, ToPrimitive, TryFromPrimitive},
};

use super::HigherRank;

// TODO: Remove when refactor the Ord for Price
impl<T> HigherRank<T> for u128
where
    T: Into<Self>,
{
    type Type = Uint256;
    type Intermediate = Uint128;
}

impl<C> Fractionable<Coin<C>> for Duration {
    type HigherPrimitive = U256;
}

impl ToPrimitive<U256> for Duration {
    fn into_primitive(self) -> U256 {
        u128::from(self.nanos()).into()
    }
}

impl TryFromPrimitive<U256> for Duration {
    fn try_from_primitive(primitive: U256) -> Option<Self> {
        u128::try_from(primitive)
            .ok()
            .and_then(|u_128| Units::try_from(u_128).ok().map(Self::from_nanos))
    }
}

#[cfg(test)]
mod tests {
    use currency::test::SuperGroupTestC1;

    use crate::{coin::Coin, duration::Duration, ratio::SimpleFraction, rational::Rational};

    #[test]
    fn of() {
        let d = Duration::from_secs(10);
        let res = SimpleFraction::new(
            Coin::<SuperGroupTestC1>::new(10),
            Coin::<SuperGroupTestC1>::new(20),
        )
        .of(d);
        assert_eq!(Duration::from_secs(5), res.unwrap());
    }

    #[test]
    fn of_max() {
        let d = Duration::from_secs(10);
        let res = SimpleFraction::new(
            Coin::<SuperGroupTestC1>::new(u128::MAX),
            Coin::<SuperGroupTestC1>::new(u128::MAX / 2),
        )
        .of(d);
        assert_eq!(Duration::from_secs(20), res.unwrap());
    }
}
