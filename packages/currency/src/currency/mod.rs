use std::{any::TypeId, fmt::Debug};

use crate::error::{Error, Result as CrateResult};

pub use self::{
    from_symbol_any::{
        visit_any_on_ticker, visit_any_on_tickers, AnyVisitor, AnyVisitorPair,
        AnyVisitorPairResult, AnyVisitorResult,
    },
    group::{Group, GroupExt, MaybeAnyVisitResult},
    matcher::{BankSymbolMatcher, DexSymbolMatcher, Matcher, MatcherExt, TickerMatcher},
};

mod from_symbol_any;
mod group;
mod matcher;

pub type SymbolUnsized = str;
pub type Symbol<'a> = &'a SymbolUnsized;
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

pub trait CurrencyExt: Currency {
    fn get_from<M>(matcher: M, field_value: &M::FieldType) -> Option<Self>
    where
        M: MatcherExt,
    {
        matcher.match_field(field_value)
    }

    fn get_from_ticker(ticker: &<TickerMatcher as Matcher>::FieldType) -> Option<Self> {
        Self::get_from(TickerMatcher, ticker)
    }

    fn get_from_bank_symbol(
        bank_symbol: &<BankSymbolMatcher as Matcher>::FieldType,
    ) -> Option<Self> {
        Self::get_from(BankSymbolMatcher, bank_symbol)
    }

    fn get_from_dex_symbol(dex_symbol: &<DexSymbolMatcher as Matcher>::FieldType) -> Option<Self> {
        Self::get_from(DexSymbolMatcher, dex_symbol)
    }

    fn visit<V>(visitor: V) -> VisitorResult<Self, V>
    where
        V: SingleVisitor<Self>,
    {
        visitor.on()
    }
}

impl<T> CurrencyExt for T where T: Currency + ?Sized {}

pub fn equal<C1, C2>() -> bool
where
    C1: 'static,
    C2: 'static,
{
    TypeId::of::<C1>() == TypeId::of::<C2>()
}

pub fn validate<G>(ticker: Symbol<'_>) -> CrateResult<()>
where
    G: GroupExt,
{
    struct SupportedLeaseCurrency {}

    impl AnyVisitor for SupportedLeaseCurrency {
        type Output = ();

        type Error = Error;

        fn on<C>(self) -> CrateResult<Self::Output>
        where
            C: Currency,
        {
            Ok(())
        }
    }

    visit_any_on_ticker::<G, _>(ticker, SupportedLeaseCurrency {})
}

pub trait SingleVisitor<C> {
    type Output;

    type Error;

    fn on(self) -> VisitorResult<C, Self>;
}

pub type VisitorResult<C, V> =
    Result<<V as SingleVisitor<C>>::Output, <V as SingleVisitor<C>>::Error>;

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
