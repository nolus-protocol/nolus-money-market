use std::{any::TypeId, fmt::Debug};

pub use crate::{
    dto::{dto, symbol, to_string, CurrencyDTO},
    from_symbol::{CurrencyVisit, SingleVisitor},
    from_symbol_any::{
        visit_any_on_currencies, AnyVisitor, AnyVisitorPair, AnyVisitorPairResult,
        AnyVisitorResult, GroupVisit,
    },
    group::{Group, MaybeAnyVisitResult, MemberOf},
    matcher::{Matcher, TypeMatcher},
    nls::{Native as NativePlatform, NlsPlatform},
    symbol::{BankSymbols, DexSymbols, Symbol, Tickers},
};

mod dto;
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
}

pub trait Definition: 'static {
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

pub fn maybe_visit_any<M, C, V>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<C::Group, V>
where
    M: Matcher<Group = C::Group>,
    C: Currency + MemberOf<V::VisitorG> + Definition,
    V: AnyVisitor<C::Group>,
    C::Group: MemberOf<V::VisitorG>,
{
    maybe_visit_member::<_, C, C::Group, _>(matcher, visitor)
}

pub fn maybe_visit_member<M, C, TopG, V>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<TopG, V>
where
    M: Matcher<Group = C::Group>,
    C: Currency + MemberOf<TopG> + MemberOf<V::VisitorG> + Definition,
    C::Group: MemberOf<TopG>,
    V: AnyVisitor<TopG>,
    TopG: Group + MemberOf<V::VisitorG>,
{
    if matcher.r#match::<C>() {
        Ok(visitor.on::<C>())
    } else {
        Err(visitor)
    }
}

pub fn visit_noone<VisitedG, V>(visitor: V) -> MaybeAnyVisitResult<VisitedG, V>
where
    VisitedG: Group + MemberOf<V::VisitorG>,
    V: AnyVisitor<VisitedG>,
{
    Err(visitor)
}
