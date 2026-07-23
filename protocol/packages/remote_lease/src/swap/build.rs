use currency::{CurrencyDef, Group, MemberOf};
use finance::coin::{Coin, CoinDTO, WithCoin};

use crate::{error::Error, swap::SwapParams};

const MAX_IN_COINS: usize = 2;

// A builder of swap parameters
//
// Unifies coins of the same currency into a single input, so e.g.,
// an Lpn downpayment plus an Lpn loan yield a single-input swap
// rather than a rejected two-input one.
#[derive(Default)]
pub struct Builder<GIn, OutC>
where
    GIn: Group,
{
    in_coins: Vec<CoinDTO<GIn>>,
    min_out: Coin<OutC>,
}

impl<GIn, OutC> Builder<GIn, OutC>
where
    GIn: Group,
{
    pub fn new() -> Self {
        Self::new_internal(Vec::with_capacity(MAX_IN_COINS), Coin::default())
    }

    fn new_internal(in_coins: Vec<CoinDTO<GIn>>, min_out: Coin<OutC>) -> Self {
        Self { in_coins, min_out }
    }

    pub fn add_coin(mut self, coin_in: CoinDTO<GIn>, min_out: Coin<OutC>) -> Option<Self> {
        match self
            .in_coins
            .iter()
            .position(|existing| existing.currency() == coin_in.currency())
        {
            Some(pos) => self.in_coins[pos] = merge_same_currency(self.in_coins[pos], coin_in)?,
            None => self.in_coins.push(coin_in),
        }
        self.min_out
            .checked_add(min_out)
            .map(|new_min_out| Self::new_internal(self.in_coins, new_min_out))
    }

    pub fn build<SuperG>(self) -> Result<SwapParams<SuperG, SuperG>, Error>
    where
        OutC: CurrencyDef,
        OutC::Group: MemberOf<SuperG>,
        GIn: MemberOf<SuperG>,
        SuperG: Group,
    {
        let min_out = self.min_out.into();
        match self.in_coins.len() {
            1 => SwapParams::one(self.in_coins[0].into_super_group(), min_out),
            2 => SwapParams::two(
                self.in_coins[0].into_super_group(),
                self.in_coins[1].into_super_group(),
                min_out,
            ),
            _ => unreachable!(
                "Swap with {} input coins is not supported",
                self.in_coins.len()
            ),
        }
    }
}

fn merge_same_currency<G>(base: CoinDTO<G>, addend: CoinDTO<G>) -> Option<CoinDTO<G>>
where
    G: Group,
{
    struct Merge<G>
    where
        G: Group,
    {
        addend: CoinDTO<G>,
    }

    impl<G> WithCoin<G> for Merge<G>
    where
        G: Group,
    {
        type Outcome = Option<CoinDTO<G>>;

        fn on<C>(self, base: Coin<C>) -> Self::Outcome
        where
            C: CurrencyDef,
            C::Group: MemberOf<G> + MemberOf<G::TopG>,
        {
            base.checked_add(self.addend.as_specific::<C, C::Group>(C::dto()))
                .map(CoinDTO::from)
        }
    }

    debug_assert_eq!(base.currency(), addend.currency());
    base.with_coin(Merge { addend })
}

#[cfg(test)]
mod tests {
    use currencies::{
        PaymentGroup,
        testing::{PaymentC1, PaymentC2, PaymentC3},
    };
    use finance::coin::{Amount, Coin};

    use crate::swap::SwapParams;

    use super::Builder;

    #[test]
    fn build_single_input() {
        let params = Builder::<PaymentGroup, PaymentC2>::new()
            .add_coin(
                Coin::<PaymentC1>::new(1000).into(),
                Coin::<PaymentC2>::new(500),
            )
            .expect("first coin fits")
            .build::<PaymentGroup>()
            .expect("valid single-input swap");

        assert_eq!(
            params,
            SwapParams::One {
                coin_in: Coin::<PaymentC1>::new(1000).into(),
                min_out: Coin::<PaymentC2>::new(500).into(),
            }
        );
    }

    #[test]
    fn build_two_inputs_sums_min_out() {
        let params = Builder::<PaymentGroup, PaymentC2>::new()
            .add_coin(
                Coin::<PaymentC1>::new(1000).into(),
                Coin::<PaymentC2>::new(300),
            )
            .expect("first coin fits")
            .add_coin(
                Coin::<PaymentC3>::new(500).into(),
                Coin::<PaymentC2>::new(200),
            )
            .expect("second coin fits")
            .build::<PaymentGroup>()
            .expect("valid two-input swap");

        assert_eq!(
            params,
            SwapParams::Two {
                coin_in_1: Coin::<PaymentC1>::new(1000).into(),
                coin_in_2: Coin::<PaymentC3>::new(500).into(),
                min_out: Coin::<PaymentC2>::new(500).into(),
            }
        );
    }

    #[test]
    fn build_two_same_currency_inputs_merge_to_one() {
        let params = Builder::<PaymentGroup, PaymentC2>::new()
            .add_coin(
                Coin::<PaymentC1>::new(1000).into(),
                Coin::<PaymentC2>::new(300),
            )
            .expect("first coin fits")
            .add_coin(
                Coin::<PaymentC1>::new(500).into(),
                Coin::<PaymentC2>::new(200),
            )
            .expect("second coin fits")
            .build::<PaymentGroup>()
            .expect("valid single-input swap after merge");

        assert_eq!(
            params,
            SwapParams::One {
                coin_in: Coin::<PaymentC1>::new(1500).into(),
                min_out: Coin::<PaymentC2>::new(500).into(),
            }
        );
    }

    #[test]
    fn add_coin_overflow_is_rejected() {
        let overflowed = Builder::<PaymentGroup, PaymentC2>::new()
            .add_coin(
                Coin::<PaymentC1>::new(1).into(),
                Coin::<PaymentC2>::new(Amount::MAX),
            )
            .expect("first coin fits")
            .add_coin(Coin::<PaymentC3>::new(1).into(), Coin::<PaymentC2>::new(1));

        assert!(overflowed.is_none());
    }

    #[test]
    fn build_zero_min_out_is_rejected() {
        let result = Builder::<PaymentGroup, PaymentC2>::new()
            .add_coin(
                Coin::<PaymentC1>::new(1000).into(),
                Coin::<PaymentC2>::new(0),
            )
            .expect("first coin fits")
            .build::<PaymentGroup>();

        assert!(result.is_err());
    }
}
