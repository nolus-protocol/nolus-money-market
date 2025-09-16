use std::{any::TypeId, fmt::Debug};

pub use crate::{
    definition::{Definition, DefinitionRef},
    dto::{CurrencyDTO, dto, to_string},
    from_symbol::{CurrencyVisit, SingleVisitor},
    from_symbol_any::GroupVisit,
    group::{FilterMapT, FindMapT, Group, MaybeAnyVisitResult, MemberOf},
    matcher::Matcher,
    pairs::{FindMapT as PairsFindMapT, MaybePairsVisitorResult, PairsGroup, PairsVisitor},
    symbol::{BankSymbols, DexSymbols, Symbol, Tickers},
    visit_any::{AnyVisitor, AnyVisitorPair, InPoolWith, visit_any_on_currencies},
};

mod definition;
mod dto;
pub mod error;
mod from_symbol;
mod from_symbol_any;
mod group;
mod matcher;
pub mod never;
mod pairs;
pub mod platform;
mod symbol;
#[cfg(any(test, feature = "testing"))]
pub mod test;
mod visit_any;

// TODO get rid of these definitions. Move some to much smaller scope, for example move SymbolOwned close to CurrencyDTO
// and SymbolStatic close to Symbols
pub type SymbolStatic = &'static str;
pub type SymbolOwned = String;

/// Currency market trait
///
/// Designed to enforce static checks at compile-time guard protecting against mismatches in financial, at al, operations.
pub trait Currency: Copy + Ord + Debug + 'static {}

/// Represent a Currency definition, including the currency group, the ticker, the symbols on Nolus and the DEX network, and the number of decimals.
///
pub trait CurrencyDef: Currency {
    /// The group this currency belongs to
    type Group: Group;

    fn dto() -> &'static CurrencyDTO<Self::Group>;

    #[cfg(any(test, feature = "testing"))]
    fn ticker() -> SymbolStatic {
        Self::dto().definition().ticker
    }

    #[cfg(any(test, feature = "testing"))]
    fn bank() -> SymbolStatic {
        Self::dto().definition().bank_symbol
    }

    #[cfg(any(test, feature = "testing"))]
    fn dex() -> SymbolStatic {
        Self::dto().definition().dex_symbol
    }
}

impl<T> Currency for T where T: CurrencyDef {}

pub fn equal<C1, C2>() -> bool
where
    C1: 'static,
    C2: 'static,
{
    TypeId::of::<C1>() == TypeId::of::<C2>()
}
