use std::{iter, option};

use currency::Group;
use finance::coin::CoinDTO;

use crate::CoinsNb;

/// The one or two input coins a swap operates on.
///
/// The design limits a swap to a single input coin or a pair, so the bound is
/// modelled as an enum — a compile-time invariant rather than a runtime length
/// check. The variants mirror the `One`/`Two` shape of the wire-level
/// `SwapParams` but enforce none of its further invariants: input currencies
/// may repeat or already equal the output currency, provided at least one of
/// them differs from it.
pub enum SwapCoins<G>
where
    G: Group,
{
    One(CoinDTO<G>),
    Two(CoinDTO<G>, CoinDTO<G>),
}

impl<G> SwapCoins<G>
where
    G: Group,
{
    pub(crate) const fn len(&self) -> CoinsNb {
        match self {
            Self::One(_) => 1,
            Self::Two(..) => 2,
        }
    }
}

impl<G> IntoIterator for SwapCoins<G>
where
    G: Group,
{
    type Item = CoinDTO<G>;
    type IntoIter = iter::Chain<iter::Once<CoinDTO<G>>, option::IntoIter<CoinDTO<G>>>;

    fn into_iter(self) -> Self::IntoIter {
        let (first, second) = match self {
            Self::One(first) => (first, None),
            Self::Two(first, second) => (first, Some(second)),
        };
        iter::once(first).chain(second)
    }
}

#[cfg(test)]
mod test {
    use currency::test::{SuperGroup, SuperGroupTestC1, SuperGroupTestC2};
    use finance::coin::{Amount, Coin, CoinDTO};

    use super::SwapCoins;

    #[test]
    fn len() {
        assert_eq!(1, SwapCoins::One(c1(5)).len());
        assert_eq!(2, SwapCoins::Two(c1(5), c2(7)).len());
    }

    #[test]
    fn into_iter_preserves_order() {
        assert_eq!(
            vec![c1(5)],
            SwapCoins::One(c1(5)).into_iter().collect::<Vec<_>>()
        );
        assert_eq!(
            vec![c1(5), c2(7)],
            SwapCoins::Two(c1(5), c2(7)).into_iter().collect::<Vec<_>>()
        );
    }

    fn c1(amount: Amount) -> CoinDTO<SuperGroup> {
        Coin::<SuperGroupTestC1>::new(amount).into()
    }

    fn c2(amount: Amount) -> CoinDTO<SuperGroup> {
        Coin::<SuperGroupTestC2>::new(amount).into()
    }
}
