use currency::{CurrencyDef, Group, MemberOf};

use crate::coin::{Amount, Coin, CoinDTO};

pub fn funds<G, C>(amount: Amount) -> CoinDTO<G>
where
    G: Group,
    C: CurrencyDef,
    C::Group: MemberOf<G>,
{
    Coin::<C>::new(amount).into()
}

pub mod coin {
    use currency::{
        Currency, CurrencyDef,
        test::{SuperGroupTestC1, SuperGroupTestC2},
    };

    use crate::coin::{Amount, Coin, WithCoin};

    #[derive(PartialEq, Eq, Debug, Clone)]
    pub struct Expect<CExp>(pub Coin<CExp>)
    where
        CExp: Currency;

    impl<CExp> WithCoin<CExp::Group> for Expect<CExp>
    where
        CExp: CurrencyDef,
    {
        type Outcome = bool;

        fn on<C>(self, coin: Coin<C>) -> Self::Outcome
        where
            C: Currency,
        {
            currency::equal::<CExp, C>() && coin == self.0.coerce_into()
        }
    }

    pub const fn coin1(amount: Amount) -> Coin<SuperGroupTestC1> {
        Coin::new(amount)
    }

    pub const fn coin2(amount: Amount) -> Coin<SuperGroupTestC2> {
        Coin::new(amount)
    }
}
