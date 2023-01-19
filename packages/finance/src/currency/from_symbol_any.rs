use serde::{de::DeserializeOwned, Serialize};

use crate::error::Error;

use super::{Currency, Group, Symbol};

use self::impl_any_tickers::FirstTickerVisitor;

pub type AnyVisitorResult<V> = Result<<V as AnyVisitor>::Output, <V as AnyVisitor>::Error>;
pub trait AnyVisitor {
    type Output;
    type Error;

    fn on<C>(self) -> AnyVisitorResult<Self>
    where
        C: Currency + Serialize + DeserializeOwned;
}
pub trait AnyVisitorPair {
    type Output;
    type Error;

    fn on<C1, C2>(self) -> Result<Self::Output, Self::Error>
    where
        C1: Currency + Serialize + DeserializeOwned,
        C2: Currency + Serialize + DeserializeOwned;
}

pub fn visit_any_on_ticker<G, V>(ticker: Symbol, visitor: V) -> Result<V::Output, V::Error>
where
    G: Group,
    V: AnyVisitor,
    Error: Into<V::Error>,
{
    G::maybe_visit_on_ticker(ticker, visitor)
        .unwrap_or_else(|_| Err(Error::not_in_currency_group::<_, G>(ticker).into()))
}

pub fn visit_any_on_tickers<G1, G2, V>(
    ticker1: Symbol,
    ticker2: Symbol,
    visitor: V,
) -> Result<V::Output, V::Error>
where
    G1: Group,
    G2: Group,
    V: AnyVisitorPair,
    Error: Into<V::Error>,
{
    visit_any_on_ticker::<G1, _>(ticker1, FirstTickerVisitor::<G2, _>::new(ticker2, visitor))
}

pub fn visit_any_on_bank_symbol<G, V>(
    bank_symbol: Symbol,
    visitor: V,
) -> Result<V::Output, V::Error>
where
    G: Group,
    V: AnyVisitor,
    Error: Into<V::Error>,
{
    G::maybe_visit_on_bank_symbol(bank_symbol, visitor)
        .unwrap_or_else(|_| Err(Error::not_in_currency_group::<_, G>(bank_symbol).into()))
}

mod impl_any_tickers {
    use std::marker::PhantomData;

    use serde::{de::DeserializeOwned, Serialize};

    use crate::{
        currency::{Currency, Group, Symbol},
        error::Error,
    };

    use super::{visit_any_on_ticker, AnyVisitor, AnyVisitorPair, AnyVisitorResult};

    pub struct FirstTickerVisitor<'a, G2, V>
    where
        G2: Group,
        V: AnyVisitorPair,
    {
        ticker2: Symbol<'a>,
        group2: PhantomData<G2>,
        visitor: V,
    }
    impl<'a, G2, V> FirstTickerVisitor<'a, G2, V>
    where
        G2: Group,
        V: AnyVisitorPair,
    {
        pub fn new(ticker2: Symbol<'a>, visitor: V) -> Self {
            Self {
                ticker2,
                group2: PhantomData::<G2>,
                visitor,
            }
        }
    }
    impl<'a, G2, V> AnyVisitor for FirstTickerVisitor<'a, G2, V>
    where
        G2: Group,
        V: AnyVisitorPair,
        Error: Into<V::Error>,
    {
        type Output = <V as AnyVisitorPair>::Output;
        type Error = <V as AnyVisitorPair>::Error;

        fn on<C1>(self) -> AnyVisitorResult<Self>
        where
            C1: Currency + Serialize + DeserializeOwned,
        {
            visit_any_on_ticker::<G2, _>(
                self.ticker2,
                SecondTickerVisitor {
                    currency1: PhantomData::<C1>,
                    visitor: self.visitor,
                },
            )
        }
    }

    struct SecondTickerVisitor<C1, V>
    where
        C1: Currency,
        V: AnyVisitorPair,
    {
        currency1: PhantomData<C1>,
        visitor: V,
    }
    impl<C1, V> AnyVisitor for SecondTickerVisitor<C1, V>
    where
        C1: Currency + Serialize + DeserializeOwned,
        V: AnyVisitorPair,
    {
        type Output = <V as AnyVisitorPair>::Output;
        type Error = <V as AnyVisitorPair>::Error;

        fn on<C2>(self) -> AnyVisitorResult<Self>
        where
            C2: Currency + Serialize + DeserializeOwned,
        {
            self.visitor.on::<C1, C2>()
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{
        currency::{Currency, Group},
        error::Error,
        test::{
            currency::{Dai, Nls, TestCurrencies, TestExtraCurrencies, Usdc},
            visitor::{Expect, ExpectPair, ExpectUnknownCurrency},
        },
    };

    #[test]
    fn visit_any() {
        let v_usdc = Expect::<Usdc>::default();
        assert_eq!(
            Ok(true),
            super::visit_any_on_ticker::<TestCurrencies, _>(Usdc::TICKER, v_usdc)
        );

        let v_nls = Expect::<Nls>::default();
        assert_eq!(
            Ok(true),
            super::visit_any_on_ticker::<TestCurrencies, _>(Nls::TICKER, v_nls)
        );

        assert_eq!(
            Err(Error::not_in_currency_group::<_, TestCurrencies>(
                Dai::BANK_SYMBOL
            )),
            super::visit_any_on_ticker::<TestCurrencies, _>(
                Dai::BANK_SYMBOL,
                ExpectUnknownCurrency
            )
        );
    }

    #[test]
    fn visit_any_unexpected() {
        const DENOM: &str = "my_fancy_coin";

        assert_eq!(
            super::visit_any_on_ticker::<TestCurrencies, _>(DENOM, ExpectUnknownCurrency),
            Err(Error::not_in_currency_group::<_, TestCurrencies>(DENOM)),
        );
    }

    #[test]
    fn visit_any_tickers() {
        visit_any_tickers_ok::<TestCurrencies, TestCurrencies, Usdc, Nls>();
        visit_any_tickers_ok::<TestExtraCurrencies, TestCurrencies, Dai, Usdc>();
        visit_any_tickers_ok::<TestCurrencies, TestCurrencies, Nls, Nls>();

        visit_any_tickers_fail::<TestCurrencies, TestCurrencies, Dai, Nls>();
    }

    fn visit_any_tickers_ok<G1, G2, C1, C2>()
    where
        G1: Group,
        G2: Group,
        C1: 'static + Currency,
        C2: 'static + Currency,
    {
        let v_c1_c2 = ExpectPair::<C1, C2>::default();
        assert_eq!(
            Ok(true),
            super::visit_any_on_tickers::<G1, G2, _>(C1::TICKER, C2::TICKER, v_c1_c2)
        );
    }

    fn visit_any_tickers_fail<G1, G2, C1, C2>()
    where
        G1: Group,
        G2: Group,
        C1: 'static + Currency,
        C2: 'static + Currency,
    {
        let v_c1_c2 = ExpectPair::<C1, C2>::default();
        assert!(super::visit_any_on_tickers::<G1, G2, _>(C1::TICKER, C2::TICKER, v_c1_c2).is_err());
    }
}
