use currency::{Currency, Group};

use crate::coin::{Coin, CoinDTO};

pub fn funds<G, C>(amount: u128) -> CoinDTO<G>
where
    G: Group,
    C: Currency,
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

    impl<CExp> WithCoin for Expect<CExp>
    where
        CExp: Currency,
    {
        type VisitedG = CExp::Group;
        type Output = bool;

        type Error = Error;

        fn on<C>(self, coin: Coin<C>) -> WithCoinResult<Self>
        where
            C: Currency,
        {
            Ok(equal::<CExp, C>() && Amount::from(coin) == self.0.into())
        }
    }
}
