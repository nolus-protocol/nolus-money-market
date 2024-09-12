use sdk::schemars;

use currency::{Matcher, MaybePairsVisitorResult, PairsGroup, PairsVisitor};

use crate::{define_currency, Native, PaymentGroup};

define_currency!(
    Nls,
    "NLS",
    "unls",
    "ibc/40A9BC802B6F2B51B3B9A6D2615EB8A9666755E987CABE978980CD6F08F31E1D", // transfer/channel-1035/unls
    Native,
    6
);

impl PairsGroup for Nls {
    type CommonGroup = PaymentGroup;

    fn maybe_visit<M, V>(_matcher: &M, visitor: V) -> MaybePairsVisitorResult<V>
    where
        M: Matcher,
        V: PairsVisitor<Pivot = Self>,
    {
        currency::visit_noone(visitor)
    }
}
