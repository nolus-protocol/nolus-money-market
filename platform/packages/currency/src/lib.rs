use std::{any::TypeId, fmt::Debug};

use group::MemberOf;

pub use crate::{
    dto::{CurrencyDTO, MaybeAnyVisitResult},
    from_symbol::{CurrencyVisit, SingleVisitor},
    from_symbol_any::{
        visit_any_on_currencies, AnyVisitor, AnyVisitorPair, AnyVisitorPairResult,
        AnyVisitorResult, GroupVisit,
    },
    group::Group,
    matcher::{BankSymbols, DexSymbols, Matcher, Symbols, Tickers, TypeMatcher},
    nls::{Native as NativePlatform, NlsPlatform},
};

mod dto;
pub mod error;
mod from_symbol;
mod from_symbol_any;
pub mod group;
mod matcher;
pub mod never;
mod nls;
#[cfg(any(test, feature = "testing"))]
pub mod test;

// TODO get rid of these definitions. Move some to much smaller scope, for example move SymbolOwned close to CurrencyDTO
// and SymbolStatic close to Symbols
pub type SymbolSlice = str;
pub type SymbolStatic = &'static SymbolSlice;
pub type SymbolOwned = String;

// TODO rename to CurrencyType
// Not extending Serialize + DeserializeOwbed since the serde derive implementations fail to
// satisfy trait bounds with regards of the lifetimes
// Foe example, https://stackoverflow.com/questions/70774093/generic-type-that-implements-deserializeowned
pub trait Currency: Copy + Ord + Default + Debug + 'static {
    type Group: Group;
}

pub fn equal<C1, C2>() -> bool
where
    C1: 'static + ?Sized,
    C2: 'static + ?Sized,
{
    TypeId::of::<C1>() == TypeId::of::<C2>()
}

pub fn maybe_visit_any<M, C, V>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<V>
where
    M: Matcher + ?Sized,
    C: Currency + MemberOf<V::VisitedG> + Symbols,
    V: AnyVisitor,
{
    if matcher.match_::<C>() {
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
