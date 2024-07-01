use currency::{group::MemberOf, Currency, Group};

use crate::coin::{Amount, Coin, CoinDTO};

pub fn funds<G, C>(amount: Amount) -> CoinDTO<G>
where
    G: Group,
    C: Currency + MemberOf<G>,
{
    Coin::<C>::new(amount).into()
}

pub mod coin {
    use currency::{equal, group::MemberOf, Currency};

    use crate::{
        coin::{Amount, Coin, WithCoin, WithCoinResult},
        error::Error,
    };

    #[derive(PartialEq, Eq, Debug, Clone)]
    pub struct Expect<CExp>(pub Coin<CExp>)
    where
        CExp: Currency;

    impl<CExp, G> WithCoin<G> for Expect<CExp>
    where
        CExp: Currency + MemberOf<G>,
    {
        type Output = bool;

        type Error = Error;

        fn on<C>(self, coin: Coin<C>) -> WithCoinResult<G, Self>
        where
            C: Currency + MemberOf<G>,
        {
            Ok(equal::<CExp, C>() && Amount::from(coin) == self.0.into())
        }
    }
}
