use std::{any::TypeId, fmt::Debug};

pub use crate::{
    definition::{Definition, DefinitionRef},
    dto::{CurrencyDTO, dto, single::expect_received as expect_exact_received, to_string},
    from_symbol::{CurrencyVisit, SingleVisitor},
    from_symbol_any::GroupVisit,
    group::{
        FilterMap as GroupFilterMap, FindMap as GroupFindMap, Group, MaybeAnyVisitResult, MemberOf,
        SubFilterAdapter, SubGroupFindAdapter,
        non_recursive_filter_map as non_recursive_group_filter_map,
        non_recursive_find_map as non_recursive_group_find_map,
    },
    matcher::Matcher,
    pairs::{
        FindMap as PairsFindMap, MaybePairsVisitorResult, PairsGroup, PairsVisitor,
        find as pairs_find,
    },
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

pub type SymbolRef<'symbol> = &'symbol str;
pub type SymbolStatic = SymbolRef<'static>;
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
