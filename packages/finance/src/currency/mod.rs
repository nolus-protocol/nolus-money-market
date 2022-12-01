use std::{any::TypeId, fmt::Debug};

pub use from_symbol::{
    maybe_visit_on_bank_symbol, maybe_visit_on_ticker, visit_on_bank_symbol, MaybeVisitResult,
    SingleVisitor,
};
pub use from_symbol_any::{visit_any_on_bank_symbol, visit_any_on_ticker, AnyVisitor};
pub use group::{Group, MaybeAnyVisitResult};

mod from_symbol;
mod from_symbol_any;
mod group;

pub type Symbol<'a> = &'a str;
pub type SymbolStatic = &'static str;
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
