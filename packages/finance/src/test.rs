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
        const SYMBOL: SymbolStatic = "uusdc";
    }
    impl Member<TestCurrencies> for Usdc {}
    impl Member<TestExtraCurrencies> for Usdc {}

    #[derive(
        Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Serialize, Deserialize,
    )]
    pub struct Nls;
    impl Currency for Nls {
        const SYMBOL: SymbolStatic = "unls";
    }
    impl Member<TestCurrencies> for Nls {}
    impl Member<TestExtraCurrencies> for Nls {}

    #[derive(
        Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Serialize, Deserialize,
    )]
    pub struct Dai;
    impl Currency for Dai {
        const SYMBOL: SymbolStatic = "udai";
    }
    impl Member<TestExtraCurrencies> for Dai {}

    pub struct TestCurrencies {}
    impl Group for TestCurrencies {
        fn resolve<V>(symbol: Symbol, visitor: V) -> Result<V::Output, V::Error>
        where
            V: AnyVisitor<Self>,
            Error: Into<V::Error>,
            Self: Sized,
        {
            match symbol {
                Usdc::SYMBOL => visitor.on::<Usdc>(),
                Nls::SYMBOL => visitor.on::<Nls>(),
                _ => Err(Error::UnknownCurrency(ToOwned::to_owned(symbol)).into()),
            }
        }
    }

    pub struct TestExtraCurrencies {}
    impl Group for TestExtraCurrencies {
        fn resolve<V>(symbol: Symbol, visitor: V) -> Result<V::Output, V::Error>
        where
            V: AnyVisitor<Self>,
            Error: Into<V::Error>,
            Self: Sized,
        {
            match symbol {
                Usdc::SYMBOL => visitor.on::<Usdc>(),
                Nls::SYMBOL => visitor.on::<Nls>(),
                Dai::SYMBOL => visitor.on::<Dai>(),
                _ => Err(Error::UnknownCurrency(ToOwned::to_owned(symbol)).into()),
            }
        }
    }
}
