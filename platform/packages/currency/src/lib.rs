use std::{any::TypeId, fmt::Debug};

pub use crate::{
    definition::{Definition, DefinitionRef},
    dto::{CurrencyDTO, dto, to_string},
    from_symbol::{CurrencyVisit, SingleVisitor},
    from_symbol_any::{
        AnyVisitor, AnyVisitorPair, AnyVisitorPairResult, GroupVisit, InPoolWith,
        visit_any_on_currencies,
    },
    group::{FilterMapT, FindMapT, Group, MaybeAnyVisitResult, MemberOf},
    matcher::{Matcher, TypeMatcher},
    pairs::{MaybePairsVisitorResult, PairsGroup, PairsVisitor, PairsVisitorResult},
    symbol::{BankSymbols, DexSymbols, Symbol, Tickers},
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

pub fn maybe_visit_any<M, C, V>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<C::Group, V>
where
    M: Matcher,
    C: CurrencyDef + PairsGroup<CommonGroup = <C::Group as Group>::TopG>,
    C::Group: MemberOf<C::Group> + MemberOf<<C::Group as Group>::TopG>,
    V: AnyVisitor<C::Group>,
{
    maybe_visit_member::<_, C, C::Group, _>(matcher, visitor)
}

// pub(crate) struct MatchThenMap<M, V, VisitedG> {
//     matcher: M,
//     v: V,
//     _visited_g: PhantomData<VisitedG>,
// }

// impl<M, V, VisitedG> MatchThenMap<M, V, VisitedG> {
//     pub fn new(matcher: M, v: V) -> Self {
//         Self {
//             matcher,
//             v,
//             _visited_g: PhantomData,
//         }
//     }
// }

// impl<M, V, VisitedG> FindMapT<VisitedG> for MatchThenMap<M, V, VisitedG>
// where
//     M: Matcher,
//     V: AnyVisitor<VisitedG>,
//     VisitedG: Group,
// {
//     type Outcome = V::Outcome;

//     fn on<C>(self, def: &CurrencyDTO<C::Group>) -> Result<Self::Outcome, Self>
//     where
//         C: CurrencyDef + PairsGroup<CommonGroup = VisitedG::TopG>,
//         C::Group: MemberOf<VisitedG> + MemberOf<VisitedG::TopG>,
//     {
//         if self.matcher.r#match(def.definition()) {
//             Ok(self.v.on::<C>(def))
//         } else {
//             Err(self)
//         }
//     }
// }

pub fn maybe_visit_member<M, C, VisitedG, V>(
    matcher: &M,
    visitor: V,
) -> MaybeAnyVisitResult<VisitedG, V>
where
    M: Matcher,
    C: CurrencyDef + PairsGroup<CommonGroup = VisitedG::TopG>,
    C::Group: MemberOf<VisitedG> + MemberOf<VisitedG::TopG>,
    V: AnyVisitor<VisitedG>,
    VisitedG: Group,
{
    let member = C::dto();
    if matcher.r#match(member.definition()) {
        Ok(visitor.on::<C>(member))
    } else {
        Err(visitor)
    }
}

pub fn maybe_visit_buddy<C, M, V>(matcher: &M, visitor: V) -> MaybePairsVisitorResult<V>
where
    M: Matcher,
    C: CurrencyDef
        + InPoolWith<V::Pivot>
        + PairsGroup<CommonGroup = <V::Pivot as PairsGroup>::CommonGroup>,
    C::Group: MemberOf<<V::Pivot as PairsGroup>::CommonGroup>,
    V: PairsVisitor,
{
    let buddy = C::dto();
    if matcher.r#match(buddy.definition()) {
        Ok(visitor.on::<C>(buddy))
    } else {
        Err(visitor)
    }
}

pub fn visit_noone<R, V>(visitor: V) -> Result<R, V> {
    Err(visitor)
}
