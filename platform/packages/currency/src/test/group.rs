use sdk::schemars::{self, JsonSchema};

use crate::{AnyVisitor, Group, Matcher, MaybeAnyVisitResult, SymbolSlice};

pub type SuperGroupTestC1 = impl_::TestC1;
pub type SuperGroupTestC2 = impl_::TestC2;
pub type SubGroupTestC1 = impl_::TestC3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, JsonSchema)]
pub enum SuperGroup {}
impl Group for SuperGroup {
    const DESCR: &'static str = "super_group";

    fn maybe_visit<M, V>(matcher: &M, symbol: &SymbolSlice, visitor: V) -> MaybeAnyVisitResult<V>
    where
        M: Matcher + ?Sized,
        V: AnyVisitor,
    {
        crate::maybe_visit_any::<_, SuperGroupTestC1, _>(matcher, symbol, visitor).or_else(
            |visitor| crate::maybe_visit_any::<_, SuperGroupTestC2, _>(matcher, symbol, visitor),
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, JsonSchema)]
pub enum SubGroup {}
impl Group for SubGroup {
    const DESCR: &'static str = "sub_group";

    fn maybe_visit<M, V>(matcher: &M, symbol: &SymbolSlice, visitor: V) -> MaybeAnyVisitResult<V>
    where
        M: Matcher + ?Sized,
        V: AnyVisitor,
    {
        SuperGroup::maybe_visit(matcher, symbol, visitor).or_else(|visitor| {
            crate::maybe_visit_any::<_, SubGroupTestC1, _>(matcher, symbol, visitor)
        })
    }
}

mod impl_ {
    use sdk::schemars::{self, JsonSchema};

    use crate::{Currency, SymbolStatic};

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, JsonSchema)]
    pub enum TestC1 {}
    impl Currency for TestC1 {
        const TICKER: SymbolStatic = "ticker#1";
        const BANK_SYMBOL: SymbolStatic = "ibc/bank_ticker#1";
        const DEX_SYMBOL: SymbolStatic = "ibc/dex_ticker#1";
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, JsonSchema)]
    pub enum TestC2 {}
    impl Currency for TestC2 {
        const TICKER: SymbolStatic = "ticker#2";
        const BANK_SYMBOL: SymbolStatic = "ibc/bank_ticker#2";
        const DEX_SYMBOL: SymbolStatic = "ibc/dex_ticker#2";
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, JsonSchema)]
    pub enum TestC3 {}
    impl Currency for TestC3 {
        const TICKER: SymbolStatic = "ticker#3";
        const BANK_SYMBOL: SymbolStatic = "ibc/bank_ticker#3";
        const DEX_SYMBOL: SymbolStatic = "ibc/dex_ticker#3";
    }
}
