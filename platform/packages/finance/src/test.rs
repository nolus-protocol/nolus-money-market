use currency::{Currency, Group, MemberOf};

use crate::coin::{Coin, CoinDTO};

pub fn funds<G, C>(amount: u128) -> CoinDTO<G>
where
    G: Group,
    C: Currency + MemberOf<G>,
{
    Coin::<C>::new(amount).into()
}

pub mod coin {
    use currency::{equal, Currency};

    use crate::{
        coin::{Amount, Coin, WithCoin, WithCoinResult},
        error::Error,
    };

    #[derive(PartialEq, Eq, Debug, Clone)]
    pub struct Expect<CExp>(pub Coin<CExp>)
    where
        CExp: Currency;

    impl<CExp> WithCoin<CExp::Group> for Expect<CExp>
    where
        CExp: Currency,
    {
        type VisitorG = CExp::Group;
        type Output = bool;

        type Error = Error;

        fn on<C>(self, coin: Coin<C>) -> WithCoinResult<Self::VisitorG, Self>
        where
            C: Currency,
        {
            Ok(equal::<CExp, C>() && Amount::from(coin) == self.0.into())
        }
    }
}
