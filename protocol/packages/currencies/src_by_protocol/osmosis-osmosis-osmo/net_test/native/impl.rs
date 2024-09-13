use currency::{Matcher, MaybePairsVisitorResult, PairsGroup, PairsVisitor};
use sdk::schemars;

use crate::{define_currency, lease::impl_mod::UsdcAxelar, Native, PaymentGroup};

define_currency!(
    Nls,
    "NLS",
    "unls",
    "ibc/D9AFCECDD361D38302AA66EB3BAC23B95234832C51D12489DC451FA2B7C72782", // transfer/channel-783/unls
    Native,
    6
);

impl PairsGroup for Nls {
    type CommonGroup = PaymentGroup;

    fn maybe_visit<M, V>(matcher: &M, visitor: V) -> MaybePairsVisitorResult<V>
    where
        M: Matcher,
        V: PairsVisitor<Pivot = Self>,
    {
        use currency::maybe_visit_buddy as maybe_visit;
        maybe_visit::<UsdcAxelar, _, _>(matcher, visitor)
    }
}
