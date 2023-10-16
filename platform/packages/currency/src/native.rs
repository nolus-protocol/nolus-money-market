use crate::{
    currency::{self, AnyVisitor, Group, MaybeAnyVisitResult},
    currency_macro::schemars,
    define_currency, define_symbol, Matcher, SymbolSlice,
};

define_symbol! {
    NLS {
        ["dev"]: {
            bank: "unls",
            /// full ibc route: transfer/channel-109/unls
            dex: "ibc/5E7589614F0B4B80D91923D15D8EB0972AAA6226F7566921F1D6A07EA0DB0D2C"
        },
        ["test"]: {
            bank: "unls",
            /// full ibc route: transfer/channel-110/unls
            dex: "ibc/95359FD9C5D15DBD7B9A6B7271F5E769776999590DE138ED62B6E89D5D010B7C"
        },
        ["main"]: {
            bank: "unls",
            /// full ibc route: transfer/channel-783/unls
            dex: "ibc/D9AFCECDD361D38302AA66EB3BAC23B95234832C51D12489DC451FA2B7C72782"
        },
    }
}
define_currency!(Nls, NLS);

#[derive(Clone, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug))]
pub struct Native {}
impl Group for Native {
    const DESCR: &'static str = "native";

    fn maybe_visit<M, V>(matcher: &M, symbol: &SymbolSlice, visitor: V) -> MaybeAnyVisitResult<V>
    where
        M: Matcher + ?Sized,
        V: AnyVisitor,
    {
        currency::maybe_visit_any::<_, Nls, _>(matcher, symbol, visitor)
    }
}