use sdk::schemars;

use currency::{Matcher, PairsGroup};

use crate::{define_currency, Native, PaymentGroup};

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

    fn maybe_visit<M, V>(_matcher: &M, visitor: V) -> currency::MaybePairsVisitorResult<V>
    where
        M: Matcher,
        V: currency::PairsVisitor<VisitedG = Self::CommonGroup>,
    {
        currency::visit_noone(visitor) // TODO
    }
}
