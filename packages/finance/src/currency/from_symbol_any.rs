use serde::{de::DeserializeOwned, Serialize};

use crate::error::Error;

use super::{Currency, Group, Symbol};

pub trait AnyVisitor {
    type Output;
    type Error;

    fn on<C>(self) -> Result<Self::Output, Self::Error>
    where
        C: 'static + Currency + Serialize + DeserializeOwned;
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

#[cfg(test)]
mod test {

    use crate::{
        currency::Currency,
        error::Error,
        test::{
            currency::{Dai, Nls, TestCurrencies, Usdc},
            visitor::{Expect, ExpectUnknownCurrency},
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
}
