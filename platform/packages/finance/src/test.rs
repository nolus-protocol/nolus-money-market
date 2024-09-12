use currency::{CurrencyDef, Group, MemberOf};

use crate::coin::{Coin, CoinDTO};

pub fn funds<G, C>(amount: u128) -> CoinDTO<G>
where
    G: Group,
    C: CurrencyDef,
    C::Group: MemberOf<G>,
{
    Coin::<C>::new(amount).into()
}

pub mod coin {
    use currency::{equal, Currency, CurrencyDef};

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
        CExp: CurrencyDef,
    {
        type Output = bool;

        type Error = Error;

        fn on<C>(self, coin: Coin<C>) -> WithCoinResult<CExp::Group, Self>
        where
            C: Currency,
        {
            Ok(equal::<CExp, C>() && Amount::from(coin) == self.0.into())
        }
    }
}
