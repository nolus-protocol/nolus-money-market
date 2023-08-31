use serde::{Serialize, Deserialize};

use crate::{
    currency::{AnyVisitor, Group, MaybeAnyVisitResult, SymbolStatic},
    define_currency, define_symbol,
    visitor::GeneralizedVisitorExt,
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
    const DESCR: SymbolStatic = "native";

    fn maybe_visit_on_by_ref<GV, V>(generalized_visitor: &GV, visitor: V) -> MaybeAnyVisitResult<V>
    where
        GV: GeneralizedVisitorExt,
        V: AnyVisitor,
    {
        generalized_visitor.maybe_visit::<Nls, V>(visitor)
    }
}
