use std::{any::TypeId, fmt::Debug, marker::PhantomData};

use crate::error::{Error, Result};

pub use self::{
    from_symbol::{CurrencyVisit, SingleVisitor},
    from_symbol_any::{
        visit_any_on_tickers, AnyVisitor, AnyVisitorPair, AnyVisitorPairResult, AnyVisitorResult,
        GroupVisit,
    },
    group::{Group, MaybeAnyVisitResult, MemberOf},
    matcher::{Matcher, TypeMatcher},
    nls::{Native as NativePlatform, NlsPlatform},
    symbol::{BankSymbols, DexSymbols, Symbol, Tickers},
};

pub mod error;
mod from_symbol;
mod from_symbol_any;
mod group;
mod matcher;
pub mod never;
mod nls;
mod symbol;
#[cfg(any(test, feature = "testing"))]
pub mod test;

// TODO get rid of these definitions. Move some to much smaller scope, for example move SymbolOwned close to CurrencyDTO
// and SymbolStatic close to Symbols
pub type SymbolSlice = str;
pub type SymbolStatic = &'static SymbolSlice;
pub type SymbolOwned = String;

// Not extending Serialize + DeserializeOwbed since the serde derive implementations fail to
// satisfy trait bounds with regards of the lifetimes
// For example, https://stackoverflow.com/questions/70774093/generic-type-that-implements-deserializeowned
pub trait Currency: Copy + Ord + Default + Debug + 'static {
    /// The group this currency belongs to
    type Group: Group;

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
}

pub fn equal<C1, C2>() -> bool
where
    C1: 'static,
    C2: 'static,
{
    TypeId::of::<C1>() == TypeId::of::<C2>()
}

pub fn validate_ticker(got: SymbolOwned, expected: SymbolStatic) -> Result<()> {
    if expected == got {
        Ok(())
    } else {
        Err(Error::currency_mismatch(expected, got))
    }
}

pub fn validate<G>(ticker: &SymbolSlice) -> Result<()>
where
    G: Group,
{
    struct SupportedLeaseCurrency<G> {
        expected_group: PhantomData<G>,
    }
    impl<G> AnyVisitor for SupportedLeaseCurrency<G>
    where
        G: Group,
    {
        type VisitedG = G;
        type Error = Error;
        type Output = ();
        fn on<C>(self) -> Result<Self::Output>
        where
            C: Currency,
        {
            Ok(())
        }
    }
    Tickers::<G>::visit_any(
        ticker,
        SupportedLeaseCurrency {
            expected_group: PhantomData::<G>,
        },
    )
}

pub fn maybe_visit_any<M, C, V>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<V>
where
    M: Matcher,
    C: Currency + MemberOf<V::VisitedG> + MemberOf<M::Group>,
    V: AnyVisitor,
{
    if matcher.r#match::<C>() {
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
            Err(Error::not_in_currency_group::<
                _,
                Tickers::<SubGroup>,
                SubGroup,
            >(SuperGroupTestC1::TICKER)),
            super::validate::<SubGroup>(SuperGroupTestC1::TICKER)
        );
        assert_eq!(Ok(()), super::validate::<SubGroup>(SubGroupTestC1::TICKER));
    }
}
