use crate::{
    coin::{Coin, CoinDTO},
    currency::Currency,
};

pub fn funds<C>(amount: u128) -> CoinDTO
where
    C: Currency,
{
    Coin::<C>::new(amount).into()
}

pub mod currency {
    use serde::{Deserialize, Serialize};

    use crate::{
        currency::{AnyVisitor, Currency, Group, Member, Symbol, SymbolStatic},
        error::Error,
    };

    #[derive(
        Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Serialize, Deserialize,
    )]
    pub struct Usdc;
    impl Currency for Usdc {
        const TICKER: SymbolStatic = "uusdc";
    }
    impl Member<TestCurrencies> for Usdc {}
    impl Member<TestExtraCurrencies> for Usdc {}

    #[derive(
        Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Serialize, Deserialize,
    )]
    pub struct Nls;
    impl Currency for Nls {
        const TICKER: SymbolStatic = "unls";
    }
    impl Member<TestCurrencies> for Nls {}
    impl Member<TestExtraCurrencies> for Nls {}

    #[derive(
        Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Serialize, Deserialize,
    )]
    pub struct Dai;
    impl Currency for Dai {
        const TICKER: SymbolStatic = "udai";
    }
    impl Member<TestExtraCurrencies> for Dai {}

    pub struct TestCurrencies {}
    pub const DESCR: &str = "test";
    impl Group for TestCurrencies {
        type ResolveError = Error;

        fn resolve<V>(symbol: Symbol, visitor: V) -> Result<V::Output, V::Error>
        where
            V: AnyVisitor<Self>,
            Error: Into<V::Error>,
        {
            match symbol {
                Usdc::TICKER => visitor.on::<Usdc>(),
                Nls::TICKER => visitor.on::<Nls>(),
                _ => Err(Error::NotInCurrencyGroup(symbol.into(), DESCR.into()).into()),
            }
        }
    }

    pub struct TestExtraCurrencies {}
    pub const DESCR_EXTRA: &str = "test_extra";
    impl Group for TestExtraCurrencies {
        type ResolveError = Error;

        fn resolve<V>(symbol: Symbol, visitor: V) -> Result<V::Output, V::Error>
        where
            V: AnyVisitor<Self>,
            Error: Into<V::Error>,
        {
            match symbol {
                Usdc::TICKER => visitor.on::<Usdc>(),
                Nls::TICKER => visitor.on::<Nls>(),
                Dai::TICKER => visitor.on::<Dai>(),
                _ => Err(Error::NotInCurrencyGroup(symbol.into(), DESCR_EXTRA.into()).into()),
            }
        }
    }
}
