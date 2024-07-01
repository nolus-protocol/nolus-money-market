use std::any::TypeId;

use serde::{Deserialize, Serialize};

use sdk::schemars::{self, JsonSchema};

use crate::{
    group::MemberOf, matcher::Matcher, AnyVisitor, Currency, Definition, Group,
    MaybeAnyVisitResult, SymbolStatic,
};

#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Serialize, Deserialize, JsonSchema,
)]
/// A 'local'-only 'protokol-independent' representation of Nls.
///
/// Intended to be used *only* until the TODO below gets done, and *only* in protokol-independent usecases:
/// - LP rewards
/// - Relayers' tips
pub struct NlsPlatform;
impl Definition for NlsPlatform {
    const TICKER: SymbolStatic = "NLS";

    const BANK_SYMBOL: SymbolStatic = "unls";

    // TODO Define trait PlatformCurrency as a super trait of Currency and
    // merge NlsPlatform and Nls
    const DEX_SYMBOL: SymbolStatic = "N/A_N/A_N/A";

    const DECIMAL_DIGITS: u8 = 6;
}

impl Currency for NlsPlatform {
    type Group = Native;
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Native(TypeId);

impl MemberOf<Self> for Native {}

impl Group for Native {
    const DESCR: &'static str = "Native";

    fn maybe_visit<M, V>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<V>
    where
        M: Matcher + ?Sized,
        V: AnyVisitor,
        Native: MemberOf<V::VisitedG>,
    {
        Self::maybe_visit_member(matcher, visitor)
    }

    fn maybe_visit_member<M, V>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<V>
    where
        M: Matcher + ?Sized,
        V: AnyVisitor,
        Self: MemberOf<V::VisitedG>,
    {
        crate::maybe_visit_any::<_, NlsPlatform, _>(matcher, visitor)
    }
}

impl From<Native> for TypeId {
    fn from(value: Native) -> Self {
        value.0
    }
}

impl From<TypeId> for Native {
    fn from(type_id: TypeId) -> Self {
        Self(type_id)
    }
}
