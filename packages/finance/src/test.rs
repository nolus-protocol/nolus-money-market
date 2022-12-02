use crate::{
    coin::{Coin, CoinDTO},
    currency::Currency,
};

pub fn funds<G, C>(amount: u128) -> CoinDTO<G>
where
    C: Currency,
{
    Coin::<C>::new(amount).into()
}

pub mod currency {
    use serde::{Deserialize, Serialize};

    use crate::currency::{AnyVisitor, Currency, Group, MaybeAnyVisitResult, Symbol, SymbolStatic};

    #[derive(
        Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Serialize, Deserialize,
    )]
    pub struct Usdc;
    impl Currency for Usdc {
        const TICKER: SymbolStatic = "uusdc";
        const BANK_SYMBOL: SymbolStatic = "ibc/uusdc";
        const DEX_SYMBOL: SymbolStatic = "ibc/dex_uusdc";
    }

    #[derive(
        Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Serialize, Deserialize,
    )]
    pub struct Nls;
    impl Currency for Nls {
        const TICKER: SymbolStatic = "unls";
        const BANK_SYMBOL: SymbolStatic = "ibc/unls";
        const DEX_SYMBOL: SymbolStatic = "ibc/dex_unls";
    }

    #[derive(
        Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Serialize, Deserialize,
    )]
    pub struct Dai;
    impl Currency for Dai {
        const TICKER: SymbolStatic = "udai";
        const BANK_SYMBOL: SymbolStatic = "ibc/udai";
        const DEX_SYMBOL: SymbolStatic = "ibc/dex_udai";
    }

    #[derive(Debug, PartialEq, Eq, Deserialize)]
    pub struct TestCurrencies {}
    impl Group for TestCurrencies {
        const DESCR: SymbolStatic = "test";

        fn maybe_visit_on_ticker<V>(symbol: Symbol, visitor: V) -> MaybeAnyVisitResult<V>
        where
            V: AnyVisitor,
        {
            match symbol {
                Usdc::TICKER => Ok(visitor.on::<Usdc>()),
                Nls::TICKER => Ok(visitor.on::<Nls>()),
                _ => Err(visitor),
            }
        }

        fn maybe_visit_on_bank_symbol<V>(symbol: Symbol, visitor: V) -> MaybeAnyVisitResult<V>
        where
            Self: Sized,
            V: AnyVisitor,
        {
            match symbol {
                Usdc::BANK_SYMBOL => Ok(visitor.on::<Usdc>()),
                Nls::BANK_SYMBOL => Ok(visitor.on::<Nls>()),
                _ => Err(visitor),
            }
        }
    }

    #[derive(Debug, PartialEq, Eq, Deserialize)]
    pub struct TestExtraCurrencies {}
    impl Group for TestExtraCurrencies {
        const DESCR: SymbolStatic = "test_extra";

        fn maybe_visit_on_ticker<V>(symbol: Symbol, visitor: V) -> MaybeAnyVisitResult<V>
        where
            V: AnyVisitor,
        {
            match symbol {
                Usdc::TICKER => Ok(visitor.on::<Usdc>()),
                Nls::TICKER => Ok(visitor.on::<Nls>()),
                Dai::TICKER => Ok(visitor.on::<Dai>()),
                _ => Err(visitor),
            }
        }

        fn maybe_visit_on_bank_symbol<V>(_: Symbol, _: V) -> MaybeAnyVisitResult<V>
        where
            Self: Sized,
            V: AnyVisitor,
        {
            unreachable!()
        }
    }
}

pub mod visitor {
    use std::marker::PhantomData;

    use crate::{
        currency::{equal, AnyVisitor, AnyVisitorPair, Currency, SingleVisitor},
        error::Error,
    };

    #[derive(Debug, PartialEq, Eq, Clone)]
    pub struct Expect<C>(PhantomData<C>);

    impl<C> Default for Expect<C> {
        fn default() -> Self {
            Self(Default::default())
        }
    }
    impl<C> AnyVisitor for Expect<C>
    where
        C: 'static,
    {
        type Output = bool;
        type Error = Error;

        fn on<Cin>(self) -> Result<Self::Output, Self::Error>
        where
            Cin: 'static,
        {
            Ok(equal::<C, Cin>())
        }
    }
    impl<C> SingleVisitor<C> for Expect<C> {
        type Output = bool;
        type Error = Error;

        fn on(self) -> Result<Self::Output, Self::Error> {
            Ok(true)
        }
    }

    pub struct ExpectUnknownCurrency;
    impl AnyVisitor for ExpectUnknownCurrency {
        type Output = bool;
        type Error = Error;

        fn on<C>(self) -> Result<Self::Output, Self::Error>
        where
            C: Currency,
        {
            unreachable!();
        }
    }

    impl<C> SingleVisitor<C> for ExpectUnknownCurrency {
        type Output = bool;
        type Error = Error;

        fn on(self) -> Result<Self::Output, Self::Error> {
            unreachable!();
        }
    }

    pub struct ExpectPair<C1, C2>(PhantomData<C1>, PhantomData<C2>);
    impl<C1, C2> Default for ExpectPair<C1, C2> {
        fn default() -> Self {
            Self(Default::default(), Default::default())
        }
    }
    impl<C1, C2> AnyVisitorPair for ExpectPair<C1, C2>
    where
        C1: 'static,
        C2: 'static,
    {
        type Output = bool;
        type Error = Error;

        fn on<C1in, C2in>(self) -> Result<Self::Output, Self::Error>
        where
            C1in: Currency,
            C2in: Currency,
        {
            Ok(equal::<C1, C1in>() && equal::<C2, C2in>())
        }
    }
}

pub mod coin {
    use crate::{
        coin::{Amount, Coin, WithCoin},
        currency::{equal, Currency},
        error::Error,
    };

    pub struct Expect<CExp>(pub Coin<CExp>)
    where
        CExp: Currency;

    impl<CExp> WithCoin for Expect<CExp>
    where
        CExp: Currency,
    {
        type Output = bool;

        type Error = Error;

        fn on<C>(&self, coin: Coin<C>) -> Result<Self::Output, Self::Error>
        where
            C: Currency,
        {
            Ok(equal::<CExp, C>() && Amount::from(coin) == self.0.into())
        }
    }
}
