use sdk::schemars::{self, JsonSchema};

use crate::{AnyVisitor, Currency, Group, Matcher, MaybeAnyVisitResult, SymbolSlice, SymbolStatic};

/// A 'local'-only 'dex-independent' representation of Nls.
///
/// Intended to be used *only* until the TODO below gets done, and *only* in dex-independent usecases:
/// - LP rewards
/// - Relayers' tips
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, JsonSchema)]
pub struct NlsPlatform;
impl Currency for NlsPlatform {
    const TICKER: SymbolStatic = "NLS";
    const BANK_SYMBOL: SymbolStatic = "unls";
    // TODO Define trait PlatformCurrency as a super trait of Currency and
    // merge NlsPlatform and Nls
    const DEX_SYMBOL: SymbolStatic = "N/A_N/A_N/A";
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, JsonSchema)]
pub struct Native {}
impl Group for Native {
    const DESCR: &'static str = "native";

    fn maybe_visit<M, V>(matcher: &M, symbol: &SymbolSlice, visitor: V) -> MaybeAnyVisitResult<V>
    where
        M: Matcher + ?Sized,
        V: AnyVisitor,
    {
        crate::maybe_visit_any::<_, NlsPlatform, _>(matcher, symbol, visitor)
    }
}
