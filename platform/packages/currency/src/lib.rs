use std::{any::TypeId, marker::PhantomData};

use serde::{de::DeserializeOwned, Serialize};

use crate::error::{Error, Result};

pub use self::{
    from_symbol::{CurrencyVisit, MaybeVisitResult, SingleVisitor},
    from_symbol_any::{
        visit_any_on_tickers, AnyVisitor, AnyVisitorPair, AnyVisitorPairResult, AnyVisitorResult,
        GroupVisit,
    },
    group::{Group, MaybeAnyVisitResult},
    matcher::{BankSymbols, DexSymbols, Matcher, Symbol, Symbols, Tickers},
    nls::{Native as NativePlatform, NlsPlatform},
};

pub mod error;
mod from_symbol;
mod from_symbol_any;
mod group;
mod matcher;
pub mod never;
mod nls;
#[cfg(any(test, feature = "testing"))]
pub mod test;

pub type SymbolSlice = str;
pub type SymbolStatic = &'static SymbolSlice;
pub type SymbolOwned = String;

pub trait BaseCurrency: Sized + 'static {}

impl<C> BaseCurrency for C where C: Currency {}

pub trait Currency: BaseCurrency {
    /// Identifier of the currency
    const TICKER: SymbolStatic;

    /// Symbol at the Nolus network used by the Cosmos-SDK modules, mainly the Banking one
    const BANK_SYMBOL: SymbolStatic;

    /// Symbol at the Dex network
    const DEX_SYMBOL: SymbolStatic;

    /// Exponent on which the whole unit was raised to get the currency's base
    /// unit represented by the trait.
    ///
    /// Example: `(10 ^ DECIMAL_DIGITS) uUSDC = 1 USDC`
    const DECIMAL_DIGITS: u8;

    const CONSTANTS: &'static Constants<Self> = &Constants {
        ticker: Self::TICKER,
        bank_symbol: Self::BANK_SYMBOL,
        dex_symbol: Self::DEX_SYMBOL,
        decimal_digits: Self::DECIMAL_DIGITS,
        _type: PhantomData,
    };
}

pub struct Constants<C>
where
    C: BaseCurrency,
{
    ticker: SymbolStatic,
    bank_symbol: SymbolStatic,
    dex_symbol: SymbolStatic,
    decimal_digits: u8,
    _type: PhantomData<C>,
}

impl<C> Constants<C>
where
    C: BaseCurrency,
{
    pub const fn ticker(&self) -> SymbolStatic {
        self.ticker
    }

    pub const fn bank_symbol(&self) -> SymbolStatic {
        self.bank_symbol
    }

    pub const fn dex_symbol(&self) -> SymbolStatic {
        self.dex_symbol
    }

    pub const fn decimal_digits(&self) -> u8 {
        self.decimal_digits
    }
}

pub fn equal<C1, C2>() -> bool
where
    C1: 'static,
    C2: 'static,
{
    TypeId::of::<C1>() == TypeId::of::<C2>()
}

pub fn validate_ticker<C>(ticker: &SymbolSlice) -> Result<()>
where
    C: Currency,
{
    if C::TICKER == ticker {
        Ok(())
    } else {
        Err(Error::unexpected_symbol::<_, Tickers, C>(ticker.to_owned()))
    }
}

pub fn validate_ticker_with_constants<C>(
    constants: &'static Constants<C>,
    ticker: &SymbolSlice,
) -> Result<()>
where
    C: BaseCurrency,
{
    if ticker == constants.ticker {
        Ok(())
    } else {
        Err(Error::unexpected_symbol_with_constants::<_, Tickers, C>(
            constants,
            ticker.to_owned(),
        ))
    }
}

pub fn validate_member<C, G>() -> Result<()>
where
    C: Currency,
    G: Group,
{
    validate::<G>(C::TICKER)
}

pub fn validate_member_with_constants<C, G>(constants: &'static Constants<C>) -> Result<()>
where
    C: BaseCurrency,
    G: Group,
{
    validate::<G>(constants.ticker)
}

pub fn validate<G>(ticker: &SymbolSlice) -> Result<()>
where
    G: Group,
{
    struct SupportedLeaseCurrency {}
    impl AnyVisitor for SupportedLeaseCurrency {
        type Error = Error;
        type Output = ();
        fn on<C>(self) -> Result<Self::Output>
        where
            C: Currency,
        {
            Ok(())
        }
    }
    Tickers.visit_any::<G, _>(ticker, SupportedLeaseCurrency {})
}

pub fn maybe_visit_any<M, C, V>(
    matcher: &M,
    symbol: &SymbolSlice,
    visitor: V,
) -> MaybeAnyVisitResult<V>
where
    M: Matcher + ?Sized,
    C: Currency + Serialize + DeserializeOwned,
    V: AnyVisitor,
{
    if matcher.match_::<C>(symbol) {
        Ok(visitor.on::<C>())
    } else {
        Err(visitor)
    }
}

pub fn visit_noone<V>(visitor: V) -> MaybeAnyVisitResult<V>
where
    V: AnyVisitor,
{
    Err(visitor)
}

#[cfg(test)]
mod tests {
    use crate::{
        error::Error,
        test::{SubGroup, SubGroupTestC1, SuperGroup, SuperGroupTestC1, SuperGroupTestC2},
        Currency, Tickers,
    };

    #[test]
    fn validate() {
        assert_eq!(
            Ok(()),
            super::validate::<SuperGroup>(SuperGroupTestC1::TICKER)
        );
        assert_eq!(
            Ok(()),
            super::validate::<SuperGroup>(SuperGroupTestC2::TICKER)
        );
        assert_eq!(
            Err(Error::not_in_currency_group::<_, Tickers, SubGroup>(
                SuperGroupTestC1::TICKER
            )),
            super::validate::<SubGroup>(SuperGroupTestC1::TICKER)
        );
        assert_eq!(Ok(()), super::validate::<SubGroup>(SubGroupTestC1::TICKER));
    }
}
