use std::{any::TypeId, fmt::Debug};

use crate::error::{Error, Result};

pub use self::{
    from_symbol::{
        maybe_visit_on_bank_symbol, maybe_visit_on_ticker, visit_on_bank_symbol, MaybeVisitResult,
        SingleVisitor,
    },
    from_symbol_any::{
        visit_any_on_bank_symbol, visit_any_on_ticker, visit_any_on_tickers, AnyVisitor,
        AnyVisitorPair, AnyVisitorPairResult, AnyVisitorResult,
    },
    group::{Group, MaybeAnyVisitResult},
    matcher::Matcher,
};

pub(crate) use group::maybe_visit;

mod from_symbol;
mod from_symbol_any;
mod group;
mod matcher;

pub type SymbolSlice = str;
pub type Symbol<'a> = &'a SymbolSlice;
pub type SymbolStatic = Symbol<'static>;
pub type SymbolOwned = String;

// Not extending Serialize + DeserializeOwbed since the serde derive implementations fail to
// satisfy trait bounds with regards of the lifetimes
// Foe example, https://stackoverflow.com/questions/70774093/generic-type-that-implements-deserializeowned
pub trait Currency: Copy + Ord + Default + Debug + 'static {
    /// Identifier of the currency
    const TICKER: SymbolStatic;

    /// Symbol at the Nolus network used by the Cosmos-SDK modules, mainly the Banking one
    const BANK_SYMBOL: SymbolStatic;

    /// Symbol at the Dex network
    const DEX_SYMBOL: SymbolStatic;
}

pub fn equal<C1, C2>() -> bool
where
    C1: 'static,
    C2: 'static,
{
    TypeId::of::<C1>() == TypeId::of::<C2>()
}

pub fn validate<G>(ticker: Symbol<'_>) -> Result<()>
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
    visit_any_on_ticker::<G, _>(ticker, SupportedLeaseCurrency {})
}

#[cfg(test)]
mod test {
    use crate::{
        error::Error,
        test::{Dai, Nls, TestCurrencies, TestExtraCurrencies, Usdc},
        Currency,
    };

    #[test]
    fn validate() {
        assert_eq!(Ok(()), super::validate::<TestCurrencies>(Usdc::TICKER));
        assert_eq!(Ok(()), super::validate::<TestCurrencies>(Nls::TICKER));
        assert_eq!(
            Err(Error::not_in_currency_group::<_, TestCurrencies>(
                Dai::TICKER
            )),
            super::validate::<TestCurrencies>(Dai::TICKER)
        );
        assert_eq!(Ok(()), super::validate::<TestExtraCurrencies>(Dai::TICKER));
    }
}
