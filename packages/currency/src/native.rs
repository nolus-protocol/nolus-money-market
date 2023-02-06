use finance::currency::{self, AnyVisitor, Group, MaybeAnyVisitResult, Symbol, SymbolStatic};

use crate::{currency_macro::schemars, define_currency, define_symbol, SingleVisitorAdapter};

define_symbol! {
    NLS {
        {
            bank: "unls",
            dex: "ibc/DEADCODEDEADCODE"
        },
        alt: {
            bank: "unls",
            dex: "ibc/DEADCODEDEADCODE"
        },
    }
}
define_currency!(Nls, NLS);

#[derive(Clone, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug))]
pub struct Native {}
impl Group for Native {
    const DESCR: SymbolStatic = "native";

    fn maybe_visit_on_ticker<V>(ticker: Symbol, visitor: V) -> MaybeAnyVisitResult<V>
    where
        V: AnyVisitor,
    {
        let v: SingleVisitorAdapter<_> = visitor.into();
        currency::maybe_visit_on_ticker::<Nls, _>(ticker, v).map_err(|v| v.0)
    }

    fn maybe_visit_on_bank_symbol<V>(bank_symbol: Symbol, visitor: V) -> MaybeAnyVisitResult<V>
    where
        Self: Sized,
        V: AnyVisitor,
    {
        let v: SingleVisitorAdapter<_> = visitor.into();
        currency::maybe_visit_on_bank_symbol::<Nls, _>(bank_symbol, v).map_err(|v| v.0)
    }
}
